@file:OptIn(ExperimentalMaterial3Api::class)

package com.crux.example.weather.ui.onboard

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Cloud
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crux.example.weather.OnboardReason
import com.crux.example.weather.R
import com.crux.example.weather.ui.theme.WeatherTheme
import androidx.hilt.navigation.compose.hiltViewModel

@Composable
fun OnboardScreen() {
    val viewModel = hiltViewModel<OnboardViewModel>()

    OnboardScreen(
        state = viewModel.state.collectAsState().value,
        onApiKeyChanged = viewModel::onApiKeyChanged,
        onSubmit = viewModel::onSubmit,
    )
}

@Composable
private fun OnboardScreen(
    state: OnboardUiState,
    onApiKeyChanged: (String) -> Unit,
    onSubmit: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(title = { })
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 32.dp),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Icon(
                imageVector = Icons.Outlined.Cloud,
                contentDescription = null,
                modifier = Modifier.size(64.dp),
                tint = MaterialTheme.colorScheme.primary,
            )
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = stringResource(R.string.onboard_title),
                style = MaterialTheme.typography.headlineMedium.copy(
                    fontWeight = FontWeight.Bold,
                ),
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = reasonMessage(state.reason),
                style = MaterialTheme.typography.bodyLarge,
                color = if (state.reason == OnboardReason.UNAUTHORIZED) {
                    MaterialTheme.colorScheme.error
                } else {
                    MaterialTheme.colorScheme.onSurfaceVariant
                },
            )
            Spacer(modifier = Modifier.height(32.dp))

            if (state.isSaving) {
                CircularProgressIndicator()
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = stringResource(R.string.onboard_saving),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                OutlinedTextField(
                    value = state.apiKey,
                    onValueChange = onApiKeyChanged,
                    label = { Text(stringResource(R.string.onboard_api_key_label)) },
                    placeholder = { Text(stringResource(R.string.onboard_api_key_placeholder)) },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth(),
                )
                Spacer(modifier = Modifier.height(24.dp))
                Button(
                    onClick = onSubmit,
                    enabled = state.canSubmit,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(stringResource(R.string.onboard_submit))
                }
            }
        }
    }
}

@Composable
private fun reasonMessage(reason: OnboardReason): String {
    return when (reason) {
        OnboardReason.WELCOME -> stringResource(R.string.onboard_reason_welcome)
        OnboardReason.UNAUTHORIZED -> stringResource(R.string.onboard_reason_unauthorized)
        OnboardReason.RESET -> stringResource(R.string.onboard_reason_reset)
    }
}

@Preview(showBackground = true)
@Composable
private fun OnboardScreenWelcomePreview() {
    WeatherTheme {
        OnboardScreen(
            state = OnboardUiState(
                reason = OnboardReason.WELCOME,
                apiKey = "",
                canSubmit = false,
                isSaving = false,
            ),
            onApiKeyChanged = {},
            onSubmit = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun OnboardScreenSavingPreview() {
    WeatherTheme {
        OnboardScreen(
            state = OnboardUiState(
                reason = OnboardReason.WELCOME,
                apiKey = "",
                canSubmit = false,
                isSaving = true,
            ),
            onApiKeyChanged = {},
            onSubmit = {},
        )
    }
}
