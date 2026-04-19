package com.crux.example.weather.core

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import android.util.Log
import androidx.core.content.ContextCompat
import com.crux.example.weather.Location
import com.google.android.gms.location.LocationServices
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

        suspend fun isLocationEnabled(): Boolean {
            Log.d(TAG, "checking location permissions")
            if (hasLocationPermissions()) {
                return true
            }
            return awaitLocationPermissionGranted()
        }

        suspend fun getLastLocation(): Location? =
            withContext(Dispatchers.IO) {
                if (!hasLocationPermissions()) {
                    Log.d(TAG, "no location permissions; returning null")
                    return@withContext null
                }
                suspendCoroutine { continuation ->
                    continuation.resumeWithLastLocation()
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
        private fun Continuation<Location?>.resumeWithLastLocation() {
            fusedLocationProviderClient.lastLocation
                .addOnSuccessListener { loc ->
                    if (loc == null) {
                        Log.d(TAG, "lastLocation returned null")
                        resume(null)
                    } else {
                        Log.d(TAG, "lastLocation: ${loc.latitude}, ${loc.longitude}")
                        resume(Location(loc.latitude, loc.longitude))
                    }
                }
                .addOnFailureListener { e ->
                    Log.w(TAG, "lastLocation failed: ${e.message}")
                    resume(null)
                }
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
            private val REQUIRED_PERMISSIONS =
                arrayOf(
                    Manifest.permission.ACCESS_COARSE_LOCATION,
                    Manifest.permission.ACCESS_FINE_LOCATION,
                )
        }
    }
