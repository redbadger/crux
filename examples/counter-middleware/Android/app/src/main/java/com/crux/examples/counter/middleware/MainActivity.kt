package com.crux.examples.counter.middleware

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.crux.examples.counter.middleware.ui.theme.CounterMiddlewareTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            CounterMiddlewareTheme {
                // The generated `Core` over the hand-written `MiddlewareBridge`:
                // the shell writes the handler and the bridge, and never
                // drives the loop itself.
                val core = viewModel<CounterViewModel>().core
                val state by core.view.collectAsState()
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        verticalArrangement = Arrangement.Center,
                        modifier = Modifier.padding(10.dp),
                    ) {
                        Text(text = "Crux Counter Middleware Example", fontSize = 30.sp, modifier = Modifier.padding(10.dp))
                        Text(text = "Rust Core, Kotlin Shell (Jetpack Compose)", modifier = Modifier.padding(10.dp))
                        Text(
                            text = state.text, color = if (state.confirmed) {
                                Color.Black
                            } else {
                                Color.Gray
                            }, modifier = Modifier.padding(10.dp)
                        )
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                            Button(
                                onClick = {
                                    core.update(Event.DECREMENT)
                                }, colors = ButtonDefaults.buttonColors(
                                    containerColor = Color.hsl(44F, 1F, 0.77F)
                                )
                            ) { Text(text = "Decrement", color = Color.DarkGray) }
                            Button(
                                onClick = {
                                    core.update(Event.INCREMENT)
                                }, colors = ButtonDefaults.buttonColors(
                                    containerColor = Color.hsl(348F, 0.86F, 0.61F)
                                )
                            ) { Text(text = "Increment", color = Color.White) }
                        }
                        Button(
                            onClick = { core.update(Event.RANDOM) },
                            colors = ButtonDefaults.buttonColors(
                                containerColor = Color.hsl(276F, 0.60F, 0.42F)
                            )
                        ) { Text(text = "I'm feeling lucky", color = Color.White) }
                    }
                }
            }
        }
    }
}
