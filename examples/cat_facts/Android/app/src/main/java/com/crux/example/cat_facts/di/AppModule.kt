package com.crux.example.cat_facts.di

import com.crux.example.cat_facts.core.Core
import com.crux.example.cat_facts.core.HttpClient
import org.koin.core.module.dsl.singleOf
import org.koin.dsl.module

val appModule =
    module {
        singleOf(::HttpClient)
        singleOf(::Core)
    }
