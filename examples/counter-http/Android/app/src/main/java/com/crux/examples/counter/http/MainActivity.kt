package com.crux.examples.counter.http

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
import com.crux.examples.counter.Event
import com.example.counter.ui.theme.CounterTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        setContent {
            CounterTheme {
                // The generated `Core` over the generated `FfiBridge`: the
                // shell writes the handler and never sees the FFI.
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
                        Text(text = "Crux Counter Example", fontSize = 30.sp, modifier = Modifier.padding(10.dp))
                        Text(text = "Rust Core, Kotlin Shell (Jetpack Compose)", modifier = Modifier.padding(10.dp))
                        Text(
                            text = state.text, color = if (state.confirmed) {
                                Color.Black
                            } else {
                                Color.Gray
                            }, modifier = Modifier.padding(10.dp)
                        )
                        state.error?.let { Text(text = it, color = Color.Red) }
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
                    }
                }
            }
        }
    }
}
