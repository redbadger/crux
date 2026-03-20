package com.crux.examples.counter.di

import com.crux.examples.counter.core.Core
import com.crux.examples.counter.core.HttpClient
import com.crux.examples.counter.core.SseClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule = module {
    singleOf(::HttpClient)
    singleOf(::SseClient)
    singleOf(::Core)
}
