@file:OptIn(ExperimentalMaterial3Api::class)

package com.crux.example.weather.ui.addfavorite

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crux.example.weather.GeocodingResponse
import com.crux.example.weather.LocalNames
import com.crux.example.weather.R
import com.crux.example.weather.ui.theme.WeatherTheme
import org.koin.androidx.compose.koinViewModel

@Composable
fun AddFavoriteScreen() {
    val viewModel = koinViewModel<AddFavoriteViewModel>()
    AddFavoriteScreen(
        addFavoriteUiState = viewModel.state.collectAsState().value,
        onSearch = viewModel::onSearch,
        onSubmit = viewModel::onSubmit,
        onCancel = viewModel::onCancel,
    )
}

@Composable
private fun AddFavoriteScreen(
    addFavoriteUiState: AddFavoriteUiState,
    onSearch: (String) -> Unit,
    onSubmit: (GeocodingResponse) -> Unit,
    onCancel: () -> Unit,
    initialSearchText: String = "",
) {
    var searchText by rememberSaveable { mutableStateOf(initialSearchText) }
    val isSearching = searchText.isNotBlank()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.add_favorite_title)) },
                navigationIcon = {
                    IconButton(onClick = onCancel) {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = "Cancel",
                        )
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(padding)
        ) {
            SearchBar(
                searchText = searchText,
                onSearchTextChanged = { value ->
                    searchText = value
                    if (value.isNotBlank()) {
                        onSearch(value)
                    }
                },
                onClear = { searchText = "" },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
            )
            Spacer(modifier = Modifier.height(8.dp))
            AddFavoriteResults(
                isSearching = isSearching,
                searchResults = addFavoriteUiState.searchResults,
                onSubmit = onSubmit,
                modifier = Modifier.fillMaxSize(),
            )
        }
    }
}

@Composable
private fun SearchBar(
    searchText: String,
    onSearchTextChanged: (String) -> Unit,
    onClear: () -> Unit,
    modifier: Modifier = Modifier,
) {
    TextField(
        value = searchText,
        onValueChange = onSearchTextChanged,
        placeholder = {
            Text(stringResource(R.string.add_favorite_search_hint))
        },
        leadingIcon = {
            Icon(
                imageVector = Icons.Outlined.Search,
                contentDescription = null,
            )
        },
        trailingIcon = {
            if (searchText.isNotEmpty()) {
                IconButton(onClick = onClear) {
                    Icon(
                        imageVector = Icons.Outlined.Close,
                        contentDescription = stringResource(R.string.action_clear_search),
                    )
                }
            }
        },
        singleLine = true,
        shape = RoundedCornerShape(12.dp),
        colors = TextFieldDefaults.colors(
            unfocusedContainerColor = MaterialTheme.colorScheme.surface,
            focusedContainerColor = MaterialTheme.colorScheme.surface,
        ),
        modifier = modifier,
    )
}

@Composable
private fun AddFavoriteResults(
    isSearching: Boolean,
    searchResults: List<GeocodingResponse>?,
    onSubmit: (GeocodingResponse) -> Unit,
    modifier: Modifier = Modifier,
) {
    Box(
        modifier = modifier,
        contentAlignment = Alignment.Center,
    ) {
        when {
            !isSearching -> {
                Text(
                    text = stringResource(R.string.add_favorite_prompt),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodyLarge,
                )
            }

            searchResults == null -> {
                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    CircularProgressIndicator()
                    Text(
                        text = stringResource(R.string.add_favorite_searching),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }

            searchResults.isEmpty() -> {
                Text(
                    text = stringResource(R.string.add_favorite_no_results),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodyLarge,
                )
            }

            else -> {
                SearchResultsList(
                    results = searchResults,
                    onSubmit = onSubmit,
                )
            }
        }
    }
}

@Composable
private fun SearchResultsList(
    results: List<GeocodingResponse>,
    onSubmit: (GeocodingResponse) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(horizontal = 16.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        results.forEach { result ->
            Card(
                onClick = { onSubmit(result) },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                colors = CardDefaults.cardColors(
                    containerColor = MaterialTheme.colorScheme.surface,
                ),
                elevation = CardDefaults.cardElevation(defaultElevation = 2.dp),
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(16.dp),
                    verticalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Text(
                        text = result.name,
                        style = MaterialTheme.typography.titleMedium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    val subtitle = result.state?.let { "$it, ${result.country}" } ?: result.country
                    Text(
                        text = subtitle,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
            }
        }
    }
}

@Preview(showBackground = true)
@Composable
private fun AddFavoriteScreenPreview() {
    WeatherTheme(darkTheme = false, dynamicColor = false) {
        AddFavoriteScreen(
            addFavoriteUiState = AddFavoriteUiState(
                searchResults = sampleGeocodingResults(),
            ),
            onSearch = {},
            onSubmit = {},
            onCancel = {},
            initialSearchText = "San",
        )
    }
}

private fun sampleGeocodingResults(): List<GeocodingResponse> {
    return listOf(
        GeocodingResponse(
            name = "San Francisco",
            localNames = LocalNames(),
            lat = 37.7749,
            lon = -122.4194,
            country = "US",
            state = "California",
        ),
        GeocodingResponse(
            name = "San Jose",
            localNames = LocalNames(),
            lat = 37.3382,
            lon = -121.8863,
            country = "US",
            state = "California",
        ),
    )
}
