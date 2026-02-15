package com.crux.example.cat_facts

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Public
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import coil.compose.rememberAsyncImagePainter
import com.crux.example.cat_facts.core.Core
import com.redbadger.catfacts.ui.theme.CatfactsTheme
import org.koin.android.ext.android.inject

class MainActivity : ComponentActivity() {
    private val core by inject<Core>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Initialize data when activity loads
        core.update(Event.GET)
        core.update(Event.GETPLATFORM)

        setContent {
            CatfactsTheme {
                val state by core.viewModel.collectAsState()

                Column(
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.Center,
                    modifier = Modifier.fillMaxSize().padding(10.dp),
                ) {
                    Icon(Icons.Filled.Public, "Platform")
                    Text(text = state.platform, modifier = Modifier.padding(10.dp))
                    Row(
                        horizontalArrangement = Arrangement.Center,
                        verticalAlignment = Alignment.CenterVertically,
                        modifier = Modifier.height(250.dp).padding(10.dp),
                    ) {
                        state.image?.let {
                            Image(
                                painter = rememberAsyncImagePainter(it.href),
                                contentDescription = "cat image",
                                modifier = Modifier.height(250.dp).fillMaxWidth(),
                            )
                        }
                    }
                    Text(text = state.fact, modifier = Modifier.padding(10.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        Button(
                            onClick = { core.update(Event.CLEAR) },
                            colors =
                                ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.error,
                                ),
                        ) { Text(text = "Clear", color = Color.White) }
                        Button(
                            onClick = { core.update(Event.GET) },
                            colors =
                                ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.primary,
                                ),
                        ) { Text(text = "Get", color = Color.White) }
                        Button(
                            onClick = { core.update(Event.FETCH) },
                            colors =
                                ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.secondary,
                                ),
                        ) { Text(text = "Fetch", color = Color.White) }
                    }
                }
            }
        }
    }
}
