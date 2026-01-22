@file:OptIn(ExperimentalMaterial3Api::class)

package com.crux.example.weather.ui.favorites

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import com.crux.example.weather.Location
import com.crux.example.weather.R
import com.crux.example.weather.ui.theme.WeatherTheme
import org.koin.androidx.compose.koinViewModel

@Composable
fun FavoritesScreen() {
    val viewModel = koinViewModel<FavoritesViewModel>()
    FavoritesScreen(
        favoritesUiState = viewModel.state.collectAsState().value,
        onNavigateHome = viewModel::onNavigateHome,
        onAddFavorite = viewModel::onAddFavorite,
        onSelectFavorite = viewModel::onSelectFavorite,
        onDeletePressed = viewModel::onDeletePressed,
        onDeleteConfirmed = viewModel::onDeleteConfirmed,
        onDeleteCancelled = viewModel::onDeleteCancelled,
    )
}

@Composable
private fun FavoritesScreen(
    favoritesUiState: FavoritesUiState,
    onNavigateHome: () -> Unit,
    onAddFavorite: () -> Unit,
    onSelectFavorite: () -> Unit,
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
                onSelectFavorite = onSelectFavorite,
                onDeletePressed = onDeletePressed,
            )

            if (favoritesUiState.deleteConfirmation != null) {
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
    onSelectFavorite: () -> Unit,
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
                    onSelectFavorite = onSelectFavorite,
                    onDeletePressed = onDeletePressed,
                )
            }
        }
    }
}

@Composable
private fun FavoriteCard(
    favorite: FavoriteItemUi,
    onSelectFavorite: () -> Unit,
    onDeletePressed: (Location) -> Unit,
) {
    Card(
        onClick = onSelectFavorite,
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
                deleteConfirmation = null,
            ),
            onNavigateHome = {},
            onAddFavorite = {},
            onSelectFavorite = {},
            onDeletePressed = {},
            onDeleteConfirmed = {},
            onDeleteCancelled = {},
        )
    }
}
