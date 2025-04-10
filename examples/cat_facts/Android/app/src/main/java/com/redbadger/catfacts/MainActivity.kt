package com.redbadger.catfacts

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
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewModelScope
import androidx.lifecycle.viewmodel.compose.viewModel
import coil.compose.rememberAsyncImagePainter
import com.crux.example.cat_facts.Event
import com.redbadger.catfacts.ui.theme.CatfactsTheme
import kotlin.jvm.optionals.getOrNull
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            CatfactsTheme {
                Surface(
                        modifier = Modifier.fillMaxSize(),
                        color = MaterialTheme.colorScheme.background
                ) { CatFacts() }
            }
        }
    }
}

class MyCore : Core() {
    init {
        viewModelScope.launch {
            update(Event.Get())
            update(Event.GetPlatform())
        }
    }
}

@Composable
fun CatFacts(core: MyCore = viewModel()) {
    val coroutineScope = rememberCoroutineScope()
    Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
            modifier = Modifier.fillMaxSize().padding(10.dp),
    ) {
        Icon(Icons.Filled.Public, "Platform")
        Text(text = core.view?.platform ?: "", modifier = Modifier.padding(10.dp))
        Row(
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.height(250.dp).padding(10.dp)
        ) {
            core.view?.image?.getOrNull()?.let {
                Image(
                        painter = rememberAsyncImagePainter(it.href),
                        contentDescription = "cat image",
                        modifier = Modifier.height(250.dp).fillMaxWidth()
                )
            }
        }
        Text(text = core.view?.fact ?: "", modifier = Modifier.padding(10.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Button(
                    onClick = { coroutineScope.launch { core.update(Event.Clear()) } },
                    colors =
                            ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.error
                            )
            ) { Text(text = "Clear", color = Color.White) }
            Button(
                    onClick = { coroutineScope.launch { core.update(Event.Get()) } },
                    colors =
                            ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.primary
                            )
            ) { Text(text = "Get", color = Color.White) }
            Button(
                    onClick = { coroutineScope.launch { core.update(Event.Fetch()) } },
                    colors =
                            ButtonDefaults.buttonColors(
                                    containerColor = MaterialTheme.colorScheme.secondary
                            )
            ) { Text(text = "Fetch", color = Color.White) }
        }
    }
}

@Preview(showBackground = true)
@Composable
fun DefaultPreview() {
    CatfactsTheme { CatFacts() }
}
