package com.crux.example.weather

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import com.crux.example.weather.core.Core
import com.crux.example.weather.core.LocationManager
import com.crux.example.weather.core.LocationManager.PermissionRequestListener
import com.crux.example.weather.ui.failed.FailedScreen
import com.crux.example.weather.ui.favorites.FavoritesScreen
import com.crux.example.weather.ui.home.HomeScreen
import com.crux.example.weather.ui.loading.LoadingScreen
import com.crux.example.weather.ui.onboard.OnboardScreen
import com.crux.example.weather.ui.theme.WeatherTheme
import kotlinx.coroutines.launch
import org.koin.android.ext.android.inject

class MainActivity : ComponentActivity() {

    private val core by inject<Core>()
    private val locationManager by inject<LocationManager>()

    private var permissionRequestListener: PermissionRequestListener? = null
    private val permissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { result ->
        permissionRequestListener?.onPermissionResult(result)
        permissionRequestListener = null
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        observeLocationPermissionRequests()

        // ANCHOR: content_view
        setContent {
            WeatherTheme {
                val state by core.viewModel.collectAsState()

                BackHandler(enabled = state is ViewModel.Active) {
                    handleBackNavigation(state)
                }

                AnimatedContent(
                    targetState = state,
                    contentKey = { it::class },
                    transitionSpec = {
                        fadeIn(animationSpec = tween(200)).togetherWith(
                            fadeOut(animationSpec = tween(200))
                        )
                    },
                ) { viewModel ->
                    when (viewModel) {
                        is ViewModel.Loading -> LoadingScreen()

                        is ViewModel.Onboard -> OnboardScreen()

                        is ViewModel.Active -> {
                            when (viewModel.value) {
                                is ActiveViewModel.Home -> HomeScreen()
                                is ActiveViewModel.Favorites -> FavoritesScreen()
                            }
                        }

                        is ViewModel.Failed -> FailedScreen(message = viewModel.message)
                    }
                }
            }
        }
        // ANCHOR_END: content_view
    }

    private fun handleBackNavigation(currentState: ViewModel) {
        val active = (currentState as? ViewModel.Active)?.value ?: return
        when (active) {
            is ActiveViewModel.Favorites -> {
                core.update(
                    Event.Active(
                        ActiveEvent.Favorites(FavoritesScreenEvent.GoToHome)
                    )
                )
            }

            is ActiveViewModel.Home -> {
                // On home screen, let the system handle back
            }
        }
    }

    private fun observeLocationPermissionRequests() {
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                locationManager.permissionRequests.collect { request ->
                    requestLocationPermission(request)
                }
            }
        }
    }

    private fun requestLocationPermission(request: LocationManager.PermissionRequest) {
        permissionRequestListener = request.listener
        permissionLauncher.launch(request.permissions)
    }
}
