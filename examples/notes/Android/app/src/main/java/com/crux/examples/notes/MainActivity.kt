package com.crux.examples.notes

import android.content.Context
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.datastore.preferences.preferencesDataStore
import androidx.lifecycle.ViewModelProvider
import com.crux.examples.notes.ui.theme.NotesTheme
import kotlinx.coroutines.launch

val Context.dataStore by preferencesDataStore(name = "notes_kv_store")

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val core = ViewModelProvider(
            this,
            object : ViewModelProvider.Factory {
                @Suppress("UNCHECKED_CAST")
                override fun <T : androidx.lifecycle.ViewModel> create(modelClass: Class<T>): T {
                    return Core(dataStore) as T
                }
            }
        )[Core::class.java]

        core.update(Event.Open)

        setContent {
            NotesTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) { NoteView(core) }
            }
        }
    }
}

@Composable
fun NoteView(core: Core) {
    val scope = rememberCoroutineScope()
    var textFieldValue by remember {
        mutableStateOf(TextFieldValue(core.view.text))
    }

    // Update text field when view changes from core
    val viewText = core.view.text
    if (textFieldValue.text != viewText) {
        textFieldValue = TextFieldValue(
            text = viewText,
            selection = textFieldValue.selection
        )
    }

    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        modifier = Modifier.fillMaxSize().padding(16.dp),
    ) {
        Text(
            text = "Notes",
            style = MaterialTheme.typography.headlineMedium,
            modifier = Modifier.padding(bottom = 16.dp)
        )
        TextField(
            value = textFieldValue,
            onValueChange = { newValue ->
                val oldText = textFieldValue.text
                val newText = newValue.text

                textFieldValue = newValue

                scope.launch {
                    if (oldText != newText) {
                        core.update(
                            Event.Replace(
                                0UL,
                                oldText.length.toULong(),
                                newText
                            )
                        )
                    }
                }
            },
            modifier = Modifier
                .fillMaxWidth()
                .weight(1f),
            label = { Text("Note") }
        )
    }
}
