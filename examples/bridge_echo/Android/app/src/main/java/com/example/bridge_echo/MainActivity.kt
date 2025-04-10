package com.example.bridge_echo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import com.crux.example.bridge_echo.Event
import com.example.bridge_echo.ui.theme.BridgeEchoTheme
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            BridgeEchoTheme {
                Surface(
                        modifier = Modifier.fillMaxSize(),
                        color = MaterialTheme.colorScheme.background
                ) { View() }
            }
        }
    }
}

@Composable
fun View(model: MyCore = viewModel()) {
    Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxSize().padding(10.dp),
    ) {
        Text(
                text = model.view?.count.toString(),
                fontSize = 30.sp,
                modifier = Modifier.padding(10.dp)
        )
    }
}

@Preview(showBackground = true)
@Composable
fun GreetingPreview() {
    BridgeEchoTheme { View() }
}

class MyCore : Core() {
    init {
        viewModelScope.launch { clock() }
        viewModelScope.launch { ticker() }
    }

    private suspend fun ticker() {
        withContext(Dispatchers.Default) {
            while (true) {
                update(Event.Tick())
            }
        }
    }

    private suspend fun clock() {
        withContext(Dispatchers.Default) {
            while (true) {
                update(Event.NewPeriod())
                delay(1000)
            }
        }
    }
}
