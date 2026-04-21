@file:OptIn(ExperimentalMaterial3Api::class)

package com.crux.example.weather.ui.favorites

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crux.example.weather.GeocodingResponse
import com.crux.example.weather.LocalNames
import com.crux.example.weather.Location
import com.crux.example.weather.R
import com.crux.example.weather.ui.theme.WeatherTheme
import androidx.hilt.navigation.compose.hiltViewModel

@Composable
fun FavoritesScreen() {
    val viewModel = hiltViewModel<FavoritesViewModel>()

    FavoritesScreen(
        favoritesUiState = viewModel.state.collectAsState().value,
        onNavigateHome = viewModel::onNavigateHome,
        onAddFavorite = viewModel::onAddFavorite,
        onDeletePressed = viewModel::onDeletePressed,
        onDeleteConfirmed = viewModel::onDeleteConfirmed,
        onDeleteCancelled = viewModel::onDeleteCancelled,
        onSearch = viewModel::onSearch,
        onSubmitFavorite = viewModel::onSubmitFavorite,
        onCancelAdd = viewModel::onCancelAdd,
    )
}

@Composable
private fun FavoritesScreen(
    favoritesUiState: FavoritesUiState,
    onNavigateHome: () -> Unit,
    onAddFavorite: () -> Unit,
    onDeletePressed: (Location) -> Unit,
    onDeleteConfirmed: () -> Unit,
    onDeleteCancelled: () -> Unit,
    onSearch: (String) -> Unit,
    onSubmitFavorite: (GeocodingResponse) -> Unit,
    onCancelAdd: () -> Unit,
) {
    val addState = favoritesUiState.workflow as? FavoritesWorkflowUiState.Add
    if (addState != null) {
        AddFavoriteContent(
            addState = addState,
            onSearch = onSearch,
            onSubmit = onSubmitFavorite,
            onCancel = onCancelAdd,
        )
    } else {
        FavoritesListContent(
            favoritesUiState = favoritesUiState,
            onNavigateHome = onNavigateHome,
            onAddFavorite = onAddFavorite,
            onDeletePressed = onDeletePressed,
            onDeleteConfirmed = onDeleteConfirmed,
            onDeleteCancelled = onDeleteCancelled,
        )
    }
}

// region Favorites List

@Composable
private fun FavoritesListContent(
    favoritesUiState: FavoritesUiState,
    onNavigateHome: () -> Unit,
    onAddFavorite: () -> Unit,
    onDeletePressed: (Location) -> Unit,
    onDeleteConfirmed: () -> Unit,
    onDeleteCancelled: () -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.favorites_title)) },
                navigationIcon = {
                    IconButton(onClick = onNavigateHome) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Outlined.ArrowBack,
                            contentDescription = null,
                        )
                    }
                },
                actions = {
                    IconButton(onClick = onAddFavorite) {
                        Icon(
                            imageVector = Icons.Outlined.Add,
                            contentDescription = stringResource(R.string.action_add_favorite),
                        )
                    }
                },
            )
        },
    ) { padding ->
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.surfaceVariant)
                .padding(padding)
        ) {
            FavoritesList(
                favorites = favoritesUiState.favorites,
                onDeletePressed = onDeletePressed,
            )

            val deleteWorkflow =
                favoritesUiState.workflow as? FavoritesWorkflowUiState.ConfirmDelete
            if (deleteWorkflow != null) {
                DeleteConfirmationDialog(
                    onDeleteConfirmed = onDeleteConfirmed,
                    onDeleteCancelled = onDeleteCancelled,
                )
            }
        }
    }
}

@Composable
private fun FavoritesList(
    favorites: List<FavoriteItemUi>,
    onDeletePressed: (Location) -> Unit,
) {
    if (favorites.isEmpty()) {
        EmptyFavoritesState(modifier = Modifier.fillMaxWidth())
    } else {
        LazyColumn(
            modifier = Modifier.fillMaxSize(),
            contentPadding = PaddingValues(vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            items(
                items = favorites,
                key = { Pair(it.location.lat, it.location.lon) },
            ) { favorite ->
                FavoriteCard(
                    favorite = favorite,
                    onDeletePressed = onDeletePressed,
                )
            }
        }
    }
}

@Composable
private fun FavoriteCard(
    favorite: FavoriteItemUi,
    onDeletePressed: (Location) -> Unit,
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 16.dp),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surface,
        ),
        elevation = CardDefaults.cardElevation(defaultElevation = 4.dp),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = favorite.name,
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            IconButton(onClick = { onDeletePressed(favorite.location) }) {
                Icon(
                    imageVector = Icons.Outlined.Delete,
                    contentDescription = stringResource(R.string.action_delete_favorite),
                    tint = MaterialTheme.colorScheme.error,
                )
            }
        }
    }
}

@Composable
private fun EmptyFavoritesState(modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .fillMaxSize()
            .padding(horizontal = 32.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = stringResource(R.string.favorites_empty),
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            style = MaterialTheme.typography.bodyLarge,
            textAlign = TextAlign.Center,
        )
    }
}

@Composable
private fun DeleteConfirmationDialog(
    onDeleteConfirmed: () -> Unit,
    onDeleteCancelled: () -> Unit,
) {
    AlertDialog(
        text = {
            Text(
                text = stringResource(R.string.favorites_delete_title),
                style = MaterialTheme.typography.titleMedium,
            )
        },
        onDismissRequest = onDeleteCancelled,
        dismissButton = {
            OutlinedButton(onClick = onDeleteCancelled) {
                Text(stringResource(R.string.action_cancel))
            }
        },
        confirmButton = {
            Button(
                onClick = onDeleteConfirmed,
                colors = ButtonDefaults.buttonColors(
                    containerColor = MaterialTheme.colorScheme.error,
                    contentColor = MaterialTheme.colorScheme.onError,
                ),
            ) {
                Text(stringResource(R.string.action_delete))
            }
        }
    )
}

// endregion

// region Add Favorite

@Composable
private fun AddFavoriteContent(
    addState: FavoritesWorkflowUiState.Add,
    onSearch: (String) -> Unit,
    onSubmit: (GeocodingResponse) -> Unit,
    onCancel: () -> Unit,
) {
    var searchText by rememberSaveable { mutableStateOf(addState.searchInput) }
    val isSearching = searchText.isNotBlank()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.add_favorite_title)) },
                navigationIcon = {
                    IconButton(onClick = onCancel) {
                        Icon(
                            imageVector = Icons.Default.Close,
                            contentDescription = stringResource(R.string.action_cancel),
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
                searchResults = addState.searchResults,
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
                    val subtitle =
                        result.state?.let { "$it, ${result.country}" } ?: result.country
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

// endregion

// region Previews

@Preview(showBackground = true)
@Composable
private fun FavoritesScreenPreview() {
    WeatherTheme {
        FavoritesScreen(
            favoritesUiState = FavoritesUiState(
                favorites = listOf(
                    FavoriteItemUi(
                        name = "Oslo",
                        location = Location(59.9139, 10.7522),
                    ),
                    FavoriteItemUi(
                        name = "Phoenix",
                        location = Location(33.4484, -112.0740),
                    ),
                ),
            ),
            onNavigateHome = {},
            onAddFavorite = {},
            onDeletePressed = {},
            onDeleteConfirmed = {},
            onDeleteCancelled = {},
            onSearch = {},
            onSubmitFavorite = {},
            onCancelAdd = {},
        )
    }
}

@Preview(showBackground = true)
@Composable
private fun AddFavoritePreview() {
    WeatherTheme {
        AddFavoriteContent(
            addState = FavoritesWorkflowUiState.Add(
                searchInput = "San",
                searchResults = listOf(
                    GeocodingResponse(
                        name = "San Francisco",
                        localNames = LocalNames(),
                        lat = 37.7749,
                        lon = -122.4194,
                        country = "US",
                        state = "California",
                    ),
                ),
                searching = false,
            ),
            onSearch = {},
            onSubmit = {},
            onCancel = {},
        )
    }
}

// endregion
