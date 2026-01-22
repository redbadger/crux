package com.crux.example.weather.di

import com.crux.example.weather.core.Core
import com.crux.example.weather.core.HttpClient
import com.crux.example.weather.core.LocationManager
import com.crux.example.weather.ui.home.HomeUiStateMapper
import com.crux.example.weather.ui.home.HomeViewModel
import org.koin.core.module.dsl.factoryOf
import org.koin.core.module.dsl.singleOf
import org.koin.core.module.dsl.viewModelOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::LocationManager)
    singleOf(::Core)

    factoryOf(::HomeUiStateMapper)
    viewModelOf(::HomeViewModel)
}
