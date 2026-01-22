package com.crux.example.weather.core

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import com.crux.example.weather.Location
import com.google.android.gms.location.LocationServices
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.withContext
import kotlin.coroutines.resume
import kotlin.coroutines.suspendCoroutine
import android.location.LocationManager as AndroidLocationManager

class LocationManager(
    private val context: Context,
) {
    private val fusedLocationProviderClient by lazy {
        LocationServices.getFusedLocationProviderClient(context)
    }

    private val androidLocationManager by lazy {
        context.getSystemService(Context.LOCATION_SERVICE) as AndroidLocationManager
    }

    private val _permissionRequests = MutableSharedFlow<PermissionRequest>(
        extraBufferCapacity = 1,
        onBufferOverflow = BufferOverflow.DROP_OLDEST,
    )
    val permissionRequests = _permissionRequests.asSharedFlow()


    fun isLocationEnabled(): Boolean {
        return LocationManagerCompat.isLocationEnabled(androidLocationManager)
    }

    suspend fun getLastLocation(): Location? = withContext(Dispatchers.IO) {
        val locationDeferred = CompletableDeferred<Location?>()

        if (!hasLocationPermission()) {
            val granted = awaitLocationPermissionGranted()
            if (granted) {
                locationDeferred.completeWithLastLocation()
            } else {
                locationDeferred.complete(null)
            }
        }

        locationDeferred.completeWithLastLocation()
        locationDeferred.await()
    }

    private fun hasLocationPermission(): Boolean {
        return ContextCompat.checkSelfPermission(
            context,
            REQUIRED_PERMISSION,
        ) == PackageManager.PERMISSION_GRANTED
    }

    private suspend fun awaitLocationPermissionGranted(): Boolean {
        return suspendCoroutine { continuation ->
            val request = PermissionRequest(REQUIRED_PERMISSION) { grants ->
                continuation.resume(grants[REQUIRED_PERMISSION] == true)
            }
            _permissionRequests.tryEmit(request)
        }
    }

    private fun CompletableDeferred<Location?>.completeWithLastLocation() {
        fusedLocationProviderClient.lastLocation
            .addOnSuccessListener {
                complete(Location(it.latitude, it.longitude))
            }
            .addOnFailureListener {
                complete(null)
            }
    }

    class PermissionRequest(
        val permission: String,
        val listener: PermissionRequestListener,
    )

    fun interface PermissionRequestListener {
        fun onPermissionResult(result: Map<String, Boolean>)
    }

    companion object {
        private const val REQUIRED_PERMISSION = Manifest.permission.ACCESS_COARSE_LOCATION
    }
}
