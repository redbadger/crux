package com.crux.example.weather.ui.onboard

import com.crux.example.weather.OnboardReason

data class OnboardUiState(
    val reason: OnboardReason,
    val apiKey: String,
    val canSubmit: Boolean,
    val isSaving: Boolean,
)
