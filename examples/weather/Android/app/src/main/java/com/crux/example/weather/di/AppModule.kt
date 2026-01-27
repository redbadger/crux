package com.crux.example.weather.di

import com.crux.example.weather.core.Core
import com.crux.example.weather.core.HttpClient
import com.crux.example.weather.core.KeyValueDataStoreDelegate
import com.crux.example.weather.core.KeyValueStore
import com.crux.example.weather.core.LocationManager
import com.crux.example.weather.ui.addfavorite.AddFavoriteUiStateMapper
import com.crux.example.weather.ui.addfavorite.AddFavoriteViewModel
import com.crux.example.weather.ui.favorites.FavoritesUiStateMapper
import com.crux.example.weather.ui.favorites.FavoritesViewModel
import com.crux.example.weather.ui.home.HomeUiStateMapper
import com.crux.example.weather.ui.home.HomeViewModel
import org.koin.core.module.dsl.factoryOf
import org.koin.core.module.dsl.singleOf
import org.koin.core.module.dsl.viewModelOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::LocationManager)
    singleOf(::KeyValueDataStoreDelegate)
    singleOf(::KeyValueStore)
    singleOf(::Core)

    factoryOf(::HomeUiStateMapper)
    viewModelOf(::HomeViewModel)

    factoryOf(::FavoritesUiStateMapper)
    viewModelOf(::FavoritesViewModel)

    factoryOf(::AddFavoriteUiStateMapper)
    viewModelOf(::AddFavoriteViewModel)
}
