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
import com.crux.example.weather.ui.addfavorite.AddFavoriteScreen
import com.crux.example.weather.ui.favorites.FavoritesScreen
import com.crux.example.weather.ui.home.HomeScreen
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

        setContent {
            WeatherTheme {
                val state by core.viewModel.collectAsState()

                BackHandler(enabled = state.workflow !is WorkflowViewModel.Home) {
                    handleBackClick(state.workflow)
                }

                AnimatedContent(
                    targetState = state.workflow,
                    contentKey = { it::class },
                    transitionSpec = {
                        fadeIn(animationSpec = tween(200)).togetherWith(
                            fadeOut(animationSpec = tween(200))
                        )
                    },
                ) { workflow ->
                    when (workflow) {
                        is WorkflowViewModel.AddFavorite -> AddFavoriteScreen()
                        is WorkflowViewModel.Favorites -> FavoritesScreen()
                        is WorkflowViewModel.Home -> HomeScreen()
                    }
                }
            }
        }
    }

    private fun handleBackClick(currentWorkflow: WorkflowViewModel) {
        when (currentWorkflow) {
            is WorkflowViewModel.AddFavorite -> core.update(
                Event.Navigate(Workflow.Home)
            )

            is WorkflowViewModel.Favorites -> core.update(
                Event.Navigate(Workflow.Home)
            )

            is WorkflowViewModel.Home -> {}
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
