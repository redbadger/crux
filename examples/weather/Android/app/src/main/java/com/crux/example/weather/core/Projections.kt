package com.crux.example.weather.core

import com.crux.example.weather.ActiveViewModel
import com.crux.example.weather.Core
import com.crux.example.weather.FavoritesViewModel
import com.crux.example.weather.HomeViewModel
import com.crux.example.weather.OnboardViewModel
import com.crux.example.weather.ViewModel
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.mapNotNull

/// The core publishes one view model for the whole app; each screen's
/// `ViewModel` narrows it to the branch it can render and ignores the rest.
fun Core.homeViewModel(): Flow<HomeViewModel> =
    view.mapNotNull { vm ->
        (vm as? ViewModel.Active)?.let { (it.value as? ActiveViewModel.Home)?.value }
    }

fun Core.favoritesViewModel(): Flow<FavoritesViewModel> =
    view.mapNotNull { vm ->
        (vm as? ViewModel.Active)?.let { (it.value as? ActiveViewModel.Favorites)?.value }
    }

fun Core.onboardViewModel(): Flow<OnboardViewModel> = view.mapNotNull { (it as? ViewModel.Onboard)?.value }
