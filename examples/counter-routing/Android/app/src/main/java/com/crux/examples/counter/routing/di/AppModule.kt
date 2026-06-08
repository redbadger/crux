package com.crux.examples.counter.routing.di

import com.crux.examples.counter.routing.core.Core
import com.crux.examples.counter.routing.core.HttpClient
import com.crux.examples.counter.routing.core.SseClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::SseClient)
    singleOf(::Core)
}
