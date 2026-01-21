package com.crux.example.weather.di

import com.crux.example.weather.core.Core
import com.crux.example.weather.core.HttpClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::Core)
}
