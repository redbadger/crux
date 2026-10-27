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

        /** True if the system location switch is on and this app has the permission. */
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

        /**
         * The device's coordinates, or `null` if there is no fix. If the cached
         * fix is empty (for example, on a fresh emulator), this asks for a live fix.
         */
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

        /**
         * High accuracy uses GPS, which is where an emulator injects its position.
         * Each continuation is resumed exactly once, here or in [resumeWithLocation].
         */
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

        /** The platform providers can have a fix when the fused client has none. */
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
