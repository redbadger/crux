package com.crux.examples.counter.middleware.di

import com.crux.examples.counter.middleware.core.Core
import com.crux.examples.counter.middleware.core.HttpClient
import com.crux.examples.counter.middleware.core.SseClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::SseClient)
    singleOf(::Core)
}
