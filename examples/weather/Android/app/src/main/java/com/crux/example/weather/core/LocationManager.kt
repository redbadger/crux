package com.crux.example.weather.core

import android.Manifest
import android.annotation.SuppressLint
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import com.crux.example.weather.Location
import com.google.android.gms.location.LocationServices
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.withContext
import kotlin.coroutines.Continuation
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine

class LocationManager(
    private val context: Context,
) {
    private val fusedLocationProviderClient by lazy {
        LocationServices.getFusedLocationProviderClient(context)
    }

    private val _permissionRequests = MutableSharedFlow<PermissionRequest>(
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    val permissionRequests = _permissionRequests.asSharedFlow()

    suspend fun isLocationEnabled(): Boolean {
        if (hasLocationPermissions()) {
            return true
        }

        return awaitLocationPermissionGranted()
    }

    suspend fun getLastLocation(): Location? = withContext(Dispatchers.IO) {
        if (!hasLocationPermissions()) {
            return@withContext null
        }

        suspendCoroutine { continuation ->
            continuation.resumeWithLastLocation()
        }
    }

    private fun hasLocationPermissions(): Boolean {
        return REQUIRED_PERMISSIONS.all {
            ContextCompat.checkSelfPermission(context, it) == PackageManager.PERMISSION_GRANTED
        }
    }

    private suspend fun awaitLocationPermissionGranted(): Boolean = withContext(Dispatchers.IO) {
        suspendCoroutine { continuation ->
            val request = PermissionRequest(REQUIRED_PERMISSIONS) { result ->
                val granted = result.all { it.value }
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
                    resume(null)
                }
                resume(Location(loc.latitude, loc.longitude))
            }
            .addOnFailureListener {
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
        private val REQUIRED_PERMISSIONS = arrayOf(
            Manifest.permission.ACCESS_COARSE_LOCATION,
            Manifest.permission.ACCESS_FINE_LOCATION,
        )
    }
}
