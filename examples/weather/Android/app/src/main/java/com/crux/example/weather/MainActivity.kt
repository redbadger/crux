package com.crux.example.weather

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.crux.example.weather.core.Core
import com.crux.example.weather.core.LocationManager
import com.crux.example.weather.ui.addfavorite.AddFavoriteScreen
import com.crux.example.weather.ui.favorites.FavoritesScreen
import com.crux.example.weather.ui.home.HomeScreen
import com.crux.example.weather.ui.theme.WeatherTheme
import kotlinx.coroutines.launch
import org.koin.android.ext.android.inject

class MainActivity : ComponentActivity() {

    private val core by inject<Core>()
    private val locationManager by inject<LocationManager>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        observeLocationPermissionRequests()

        setContent {
            WeatherTheme {
                val state by core.viewModel.collectAsState()

                // TODO: add navigation backstack
                when (state.workflow) {
                    is WorkflowViewModel.AddFavorite -> AddFavoriteScreen()
                    is WorkflowViewModel.Favorites -> FavoritesScreen()
                    is WorkflowViewModel.Home -> HomeScreen()
                }
            }
        }
    }

    private fun observeLocationPermissionRequests() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                locationManager.permissionRequests.collect { request ->
                    val permissionLauncher = registerForActivityResult(
                        ActivityResultContracts.RequestMultiplePermissions(),
                    ) { result ->
                        request.listener.onPermissionResult(result)
                    }

                    permissionLauncher.launch(arrayOf(request.permission))
                }
            }
        }
    }
}
