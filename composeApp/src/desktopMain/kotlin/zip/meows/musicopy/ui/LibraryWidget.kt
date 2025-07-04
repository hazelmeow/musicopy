package zip.meows.musicopy.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.IconButton
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.composables.core.Dialog
import com.composables.core.DialogPanel
import com.composables.core.DialogState
import com.composables.core.Scrim
import kotlinx.coroutines.launch
import musicopy.composeapp.generated.resources.Res
import musicopy.composeapp.generated.resources.cell_tower_24px
import musicopy.composeapp.generated.resources.content_copy_24px
import okio.Path.Companion.toPath
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.CoreException
import uniffi.musicopy.LibraryRootModel
import uniffi.musicopy.Model
import uniffi.musicopy.pickFolder

@Composable
fun LibraryWidget(
    model: Model,
    onAddRoot: (name: String, path: String) -> Unit,
    onRemoveRoot: (name: String) -> Unit,
    onRescan: () -> Unit,
) {
    val localRoots = model.library.localRoots

    val dialogState = com.composables.core.rememberDialogState(initiallyVisible = false)
    var pickedPath by remember { mutableStateOf<String?>(null) }
    var dialogName by remember { mutableStateOf("") }

    val scope = rememberCoroutineScope()

    val onStartAddRoot = {
        scope.launch {
            try {
                pickedPath = pickFolder()
                pickedPath?.let {
                    if (localRoots.any { root -> root.path == pickedPath }) {
                        // TODO
                        println("root already exists")
                        return@launch
                    }

                    dialogName = it.toPath(normalize = true).name
                    dialogState.visible = true
                }
            } catch (e: CoreException) {
                // TODO: toast?
                println("Error: ${e}")
            }
        }
        Unit
    }

    pickedPath?.let {
        AddRootDialog(
            state = dialogState,
            path = it,
            name = dialogName,
            setName = { it -> dialogName = it },
            onSubmit = { name, path ->
                onAddRoot(name, path)
                pickedPath = null
                dialogName = ""
                dialogState.visible = false
            },
            onCancel = {
                pickedPath = null
                dialogName = ""
                dialogState.visible = false
            },
            localRoots = localRoots
        )
    }

    Card(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(4.dp),
//            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            Box(
                modifier = Modifier.fillMaxWidth().padding(4.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth().height(48.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        "Library",
                        style = MaterialTheme.typography.titleLarge,
                        modifier = Modifier.padding(start = 8.dp)
                    )

                    Box(modifier = Modifier.weight(1f))

                    if (localRoots.isNotEmpty()) {
                        IconButton(
                            onClick = onRescan,
                            //                    modifier = Modifier.background(MaterialTheme.colorScheme.primary)
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.cell_tower_24px),
                                contentDescription = "Rescan library button",
                            )
                        }

                        IconButton(
                            onClick = onStartAddRoot,
                            //                    modifier = Modifier.background(MaterialTheme.colorScheme.primary)
                        ) {
                            Icon(
                                painter = painterResource(Res.drawable.content_copy_24px),
                                contentDescription = "Add library root button",
                            )
                        }
                    }
                }
            }

            Column(
                modifier = Modifier.fillMaxWidth(),
                verticalArrangement = Arrangement.spacedBy(4.dp)
            ) {
                if (localRoots.isNotEmpty()) {
                    for (root in localRoots) {
                        LibraryRoot(root, onRemoveRoot = onRemoveRoot)
                    }
                } else {
                    Empty(onStartAddRoot = onStartAddRoot)
                }
            }
        }
    }
}

@Composable
private fun LibraryRoot(root: LibraryRootModel, onRemoveRoot: (String) -> Unit) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = MaterialTheme.colorScheme.surfaceContainer
        )
    ) {
        Row(
            modifier = Modifier.fillMaxWidth().clip(MaterialTheme.shapes.medium)
        ) {
            Row(
                modifier = Modifier.padding(4.dp),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.padding(start = 8.dp)) {
                    Text(
                        "${root.name} (${root.numFiles})",
                        style = MaterialTheme.typography.labelLarge
                    )
                    Text("${root.path}", style = MaterialTheme.typography.labelMedium)
                }

                Box(modifier = Modifier.weight(1f))

                IconButton(
                    onClick = {
                        // TODO
                    },
                ) {
                    Icon(
                        painter = painterResource(Res.drawable.content_copy_24px),
                        contentDescription = "Open button"
                    )
                }

                IconButton(
                    onClick = { onRemoveRoot(root.name) },
                ) {
                    Icon(
                        painter = painterResource(Res.drawable.content_copy_24px),
                        contentDescription = "Remove button"
                    )
                }
            }
        }

    }
}

@Composable
private fun Empty(onStartAddRoot: () -> Unit) {

    Column(modifier = Modifier.fillMaxWidth()) {
        Text("Lorem ipsum", modifier = Modifier.padding(start = 12.dp))

        Row(
            modifier = Modifier.fillMaxWidth().padding(vertical = 16.dp),
            horizontalArrangement = Arrangement.Center
        ) {
            IconButton(
                onClick = onStartAddRoot,
            ) {
                Icon(
                    painter = painterResource(Res.drawable.content_copy_24px),
                    contentDescription = "Add library root button",
                )
            }
        }
    }
}

// TODO: support more characters in root names

@Composable
private fun AddRootDialog(
    state: DialogState,
    path: String,
    name: String,
    setName: (String) -> Unit,
    onSubmit: (name: String, path: String) -> Unit,
    onCancel: () -> Unit,
    localRoots: List<LibraryRootModel>,
) {
    val isEmpty = name.isEmpty()
    val isTaken = localRoots.any { item -> item.name.lowercase() == name.lowercase() }
    val isValidAlphabet = name.all { c -> c.isLetterOrDigit() || c == ' ' || c == '_' || c == '-' }
    val isValid = !isEmpty && !isTaken && isValidAlphabet
    val isError = !isEmpty && !isValid
    val supportingText = when {
        isEmpty -> ""
        isTaken -> "Name is in use."
        !isValidAlphabet -> "Name contains invalid characters."
        else -> ""
    }

    Dialog(state = state, onDismiss = onCancel) {
        Scrim()
        DialogPanel(
            modifier = Modifier
                .widthIn(max = 500.dp)
                .padding(16.dp)
        ) {
            Card(
                modifier = Modifier
                    .fillMaxWidth(),
            ) {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(32.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    Text(
                        text = "Add folder to library",
                        style = MaterialTheme.typography.headlineSmall
                    )

                    Text(
                        text = buildAnnotatedString {
                            append("Choose a short name for ")
                            withStyle(style = SpanStyle(fontWeight = FontWeight.Bold)) {
                                append(path)
                            }
                            append(".")
                        },
                        style = MaterialTheme.typography.bodyMedium
                    )

                    OutlinedTextField(
                        value = name,
                        onValueChange = setName,
                        label = {
                            Text("Name")
                        },
                        maxLines = 1,
                        modifier = Modifier.fillMaxWidth(),
                        isError = isError,
                        supportingText = { Text(supportingText) }
                    )

                    Row(
                        modifier = Modifier
                            .fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(16.dp, Alignment.End),
                    ) {
                        TextButton(
                            onClick = onCancel,
                        ) {
                            Text("Cancel")
                        }

                        TextButton(
                            onClick = { onSubmit(name, path) },
                            enabled = isValid
                        ) {
                            Text("Add")
                        }
                    }
                }
            }
        }
    }
}
