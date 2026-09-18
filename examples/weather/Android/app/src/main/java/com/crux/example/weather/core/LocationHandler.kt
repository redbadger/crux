package com.crux.example.weather.core

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationManager
import android.util.Log
import androidx.core.content.ContextCompat
import com.crux.example.weather.Location
import com.google.android.gms.location.LocationServices
import com.google.android.gms.location.Priority
import com.google.android.gms.tasks.CancellationTokenSource
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.withContext
import javax.inject.Inject
import javax.inject.Singleton
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

@Singleton
class LocationHandler
    @Inject
    constructor(
        @ApplicationContext private val context: Context,
    ) {
        private val fusedLocationProviderClient by lazy {
            LocationServices.getFusedLocationProviderClient(context)
        }

        private val _permissionRequests =
            MutableSharedFlow<PermissionRequest>(
                extraBufferCapacity = 1,
                onBufferOverflow = BufferOverflow.DROP_OLDEST,
            )
        val permissionRequests = _permissionRequests.asSharedFlow()

        /// Whether the shell can locate the device: the system switch is on
        /// *and* this app may use it.
        ///
        /// Both have to hold, and they fail differently — the switch is the
        /// user's device-wide choice, the permission is this app's. The core
        /// asks one question, so a `false` here covers either, and the UI
        /// offers a retry once the user has fixed whichever it was.
        suspend fun isLocationEnabled(): Boolean {
            val manager = context.getSystemService(LocationManager::class.java)
            if (manager?.isLocationEnabled != true) {
                Log.d(TAG, "location services are switched off")
                return false
            }
            Log.d(TAG, "checking location permissions")
            if (hasLocationPermissions()) {
                return true
            }
            return awaitLocationPermissionGranted()
        }

        /// The device's coordinates, or `null` if the shell could not get a
        /// fix.
        ///
        /// The cached fix is tried first because it is free, but it is only a
        /// cache: on a device that has not located itself recently — a fresh
        /// emulator, most of all — it is empty, and a location set in the
        /// emulator's controls does not fill it. So an empty cache asks for a
        /// live fix rather than reporting failure.
        suspend fun getLocation(): Location? =
            withContext(Dispatchers.IO) {
                if (!hasLocationPermissions()) {
                    Log.d(TAG, "no location permissions; returning null")
                    return@withContext null
                }
                suspendCoroutine { continuation ->
                    continuation.resumeWithLocation()
                }
            }

        private fun hasLocationPermissions(): Boolean =
            REQUIRED_PERMISSIONS.all {
                ContextCompat.checkSelfPermission(context, it) == PackageManager.PERMISSION_GRANTED
            }

        private suspend fun awaitLocationPermissionGranted(): Boolean =
            withContext(Dispatchers.IO) {
                suspendCoroutine { continuation ->
                    val request =
                        PermissionRequest(REQUIRED_PERMISSIONS) { result ->
                            val granted = result.all { it.value }
                            Log.d(TAG, "permission result: granted=$granted")
                            continuation.resume(granted)
                        }
                    _permissionRequests.tryEmit(request)
                }
            }

        @SuppressLint("MissingPermission")
        private fun Continuation<Location?>.resumeWithLocation() {
            fusedLocationProviderClient.lastLocation
                .addOnSuccessListener { loc ->
                    if (loc == null) {
                        Log.d(TAG, "no cached fix; asking for a current one")
                        resumeWithCurrentLocation()
                    } else {
                        Log.d(TAG, "lastLocation: ${loc.latitude}, ${loc.longitude}")
                        resume(Location(loc.latitude, loc.longitude))
                    }
                }
                .addOnFailureListener { e ->
                    Log.w(TAG, "lastLocation failed: ${e.message}; asking for a current one")
                    resumeWithCurrentLocation()
                }
        }

        /// Asks the device to locate itself now, which is what answers on a
        /// device with an empty cache. Each continuation is resumed exactly
        /// once, here or in [resumeWithLocation].
        ///
        /// High accuracy, because that is the setting that uses GPS. The
        /// balanced setting locates from wifi and cell towers, which a device
        /// with neither — an emulator — cannot do, and an emulator's position
        /// is injected into the GPS provider.
        @SuppressLint("MissingPermission")
        private fun Continuation<Location?>.resumeWithCurrentLocation() {
            fusedLocationProviderClient
                .getCurrentLocation(
                    Priority.PRIORITY_HIGH_ACCURACY,
                    CancellationTokenSource().token,
                ).addOnSuccessListener { loc ->
                    if (loc == null) {
                        Log.d(TAG, "getCurrentLocation returned null; asking the platform")
                        resume(lastKnownFromPlatform())
                    } else {
                        Log.d(TAG, "currentLocation: ${loc.latitude}, ${loc.longitude}")
                        resume(Location(loc.latitude, loc.longitude))
                    }
                }.addOnFailureListener { e ->
                    Log.w(TAG, "getCurrentLocation failed: ${e.message}; asking the platform")
                    resume(lastKnownFromPlatform())
                }
        }

        /// The platform's own last fix, asked provider by provider.
        ///
        /// Play services is one layer above the providers and can decline to
        /// answer when it has nothing recent of its own. The platform manager
        /// is the layer the emulator injects into, so it answers when the
        /// fused client will not.
        @SuppressLint("MissingPermission")
        private fun lastKnownFromPlatform(): Location? {
            val manager = context.getSystemService(LocationManager::class.java) ?: return null
            for (provider in PROVIDERS) {
                val fix =
                    try {
                        manager.getLastKnownLocation(provider)
                    } catch (e: SecurityException) {
                        Log.w(TAG, "$provider refused: ${e.message}")
                        null
                    } catch (e: IllegalArgumentException) {
                        Log.d(TAG, "$provider is not available: ${e.message}")
                        null
                    }
                if (fix != null) {
                    Log.d(TAG, "$provider: ${fix.latitude}, ${fix.longitude}")
                    return Location(fix.latitude, fix.longitude)
                }
            }
            Log.d(TAG, "no provider had a fix")
            return null
        }

        class PermissionRequest(
            val permissions: Array<String>,
            val listener: PermissionRequestListener,
        )

        fun interface PermissionRequestListener {
            fun onPermissionResult(result: Map<String, Boolean>)
        }

        companion object {
            private const val TAG = "LocationHandler"
            private val PROVIDERS =
                listOf(
                    LocationManager.GPS_PROVIDER,
                    LocationManager.NETWORK_PROVIDER,
                    LocationManager.PASSIVE_PROVIDER,
                )
            private val REQUIRED_PERMISSIONS =
                arrayOf(
                    Manifest.permission.ACCESS_COARSE_LOCATION,
                    Manifest.permission.ACCESS_FINE_LOCATION,
                )
        }
    }
