package app.musicopy.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import app.musicopy.formatSize
import app.musicopy.openDirectoryInExplorer
import app.musicopy.rememberPoll
import app.musicopy.ui.components.WidgetContainer
import musicopy_root.musicopy.generated.resources.Res
import musicopy_root.musicopy.generated.resources.delete_sweep_24px
import musicopy_root.musicopy.generated.resources.folder_open_24px
import org.jetbrains.compose.resources.painterResource
import uniffi.musicopy.Model

@Composable
fun SettingsWidget(
    model: Model,
) {
    WidgetContainer(
        title = "OPTIONS",
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Card(
                modifier = Modifier.fillMaxWidth(),
            ) {
                Row(
                    modifier = Modifier.padding(4.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column(
                        modifier = Modifier.padding(start = 8.dp).weight(1f)
                    ) {
                        Text(
                            text = "Transcodes cache",
                            style = MaterialTheme.typography.labelLarge,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        val count by rememberPoll(1000) {
                            model.library.transcodeCountQueued.get() +
                                    model.library.transcodeCountInprogress.get() +
                                    model.library.transcodeCountReady.get()
                        }
                        Text(
                            text = "$count files, ${
                                formatSize(
                                    model.library.transcodesDirSize
                                )
                            }",
                            style = MaterialTheme.typography.labelMedium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }

                    IconButton(
                        onClick = {
                            openDirectoryInExplorer(model.library.transcodesDir)
                        },
                    ) {
                        Icon(
                            painter = painterResource(Res.drawable.folder_open_24px),
                            contentDescription = "Open button"
                        )
                    }

                    IconButton(
                        onClick = {
                            // TODO
                        },
                    ) {
                        Icon(
                            painter = painterResource(Res.drawable.delete_sweep_24px),
                            contentDescription = "Clean button"
                        )
                    }
                }
            }
        }
    }
}
