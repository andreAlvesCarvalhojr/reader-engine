package com.readerengine.app

import android.content.Context
import android.content.SharedPreferences
import android.content.res.AssetManager
import android.net.Uri
import android.os.Bundle
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalLifecycleOwner
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import uniffi.reader_ffi.FfiReaderSession
import uniffi.reader_ffi.ReaderEngine
import uniffi.reader_ffi.ReaderSnapshot
import uniffi.reader_ffi.SavedPosition
import uniffi.reader_ffi.SearchResultItem
import java.io.ByteArrayInputStream
import androidx.compose.ui.Modifier as ComposeModifier

private const val BOOK_BASE_URL = "book://content/"

/** Manages reading position persistence using SharedPreferences. */
class ReadingPositionStore(context: Context) {
    private val prefs: SharedPreferences =
        context.getSharedPreferences("reading_positions", Context.MODE_PRIVATE)

    fun save(bookId: String, position: SavedPosition) {
        prefs.edit()
            .putInt("${bookId}_spine_index", position.spineIndex.toInt())
            .putFloat("${bookId}_progression", position.progression)
            .apply()
    }

    fun load(bookId: String): SavedPosition? {
        if (!prefs.contains("${bookId}_spine_index")) return null
        return SavedPosition(
            spineIndex = prefs.getInt("${bookId}_spine_index", 0).toUInt(),
            progression = prefs.getFloat("${bookId}_progression", 0f),
        )
    }
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val engine = ReaderEngine()
        val samples = bundledSamples() + localSamples(assets)
        setContent {
            MaterialTheme(
                colorScheme = darkColorScheme(
                    primary = Color(0xFF3ECFAC),
                    onPrimary = Color(0xFF04201A),
                    background = Color(0xFF071512),
                    surface = Color(0xFF0C2420),
                    onBackground = Color(0xFFD7EFE7),
                    onSurface = Color(0xFFD7EFE7),
                ),
            ) {
                Surface(modifier = ComposeModifier.fillMaxSize()) {
                    ReaderScreen(
                        engine = engine,
                        samples = samples,
                        readAsset = { path -> assets.open(path).use { it.readBytes() } },
                        readUri = { uri ->
                            contentResolver.openInputStream(uri)?.use { it.readBytes() }
                                ?: error("nao foi possivel ler $uri")
                        },
                        displayName = { uri ->
                            uri.lastPathSegment ?: "book.bin"
                        },
                    )
                }
            }
        }
    }
}

private data class SampleBook(
    val id: String,
    val title: String,
    val assetPath: String,
    val fileName: String,
)

private fun bundledSamples(): List<SampleBook> = listOf(
    SampleBook("noite-clara", "Noite Clara", "samples/noite-clara.json", "noite-clara.json"),
    SampleBook("epub-fixture", "EPUB Fixture", "samples/sample.epub", "sample.epub"),
    SampleBook("epub-with-image", "EPUB With Image", "samples/sample-with-image.epub", "sample-with-image.epub"),
)

/** Amostras em `assets/samples/local/` (gitignored; uso local de teste). */
private fun localSamples(assets: AssetManager): List<SampleBook> {
    val names = runCatching { assets.list("samples/local") }
        .getOrNull()
        .orEmpty()
        .filter { it.endsWith(".json", ignoreCase = true) || it.endsWith(".epub", ignoreCase = true) }
        .sorted()
    return names.map { name ->
        val stem = name.substringBeforeLast('.')
        SampleBook(
            id = "local-$stem",
            title = stem,
            assetPath = "samples/local/$name",
            fileName = name,
        )
    }
}

private class BookResourceWebViewClient(
    private val sessionProvider: () -> FfiReaderSession?,
) : WebViewClient() {

    override fun shouldInterceptRequest(
        view: WebView?,
        request: WebResourceRequest?,
    ): WebResourceResponse? {
        val url = request?.url ?: return null
        if (url.scheme != "book" || url.host != "content") {
            return super.shouldInterceptRequest(view, request)
        }

        val session = sessionProvider() ?: return notFoundResponse()
        val href = url.path?.removePrefix("/") ?: return notFoundResponse()

        return try {
            val bytes = session.readResource(href)
            val mimeType = session.resourceMediaType(href)
                ?: guessMimeType(href)
            WebResourceResponse(mimeType, "UTF-8", ByteArrayInputStream(bytes))
        } catch (e: Exception) {
            notFoundResponse()
        }
    }

    private fun notFoundResponse() = WebResourceResponse(
        "text/plain",
        "UTF-8",
        404,
        "Not Found",
        emptyMap(),
        ByteArrayInputStream("Resource not found".toByteArray()),
    )

    private fun guessMimeType(href: String): String = when {
        href.endsWith(".html") || href.endsWith(".xhtml") -> "application/xhtml+xml"
        href.endsWith(".css") -> "text/css"
        href.endsWith(".js") -> "application/javascript"
        href.endsWith(".png") -> "image/png"
        href.endsWith(".jpg") || href.endsWith(".jpeg") -> "image/jpeg"
        href.endsWith(".gif") -> "image/gif"
        href.endsWith(".svg") -> "image/svg+xml"
        href.endsWith(".webp") -> "image/webp"
        href.endsWith(".woff") -> "font/woff"
        href.endsWith(".woff2") -> "font/woff2"
        href.endsWith(".ttf") -> "font/ttf"
        href.endsWith(".otf") -> "font/otf"
        else -> "application/octet-stream"
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ReaderScreen(
    engine: ReaderEngine,
    samples: List<SampleBook>,
    readAsset: (String) -> ByteArray,
    readUri: (Uri) -> ByteArray,
    displayName: (Uri) -> String,
) {
    val context = LocalContext.current
    val positionStore = remember { ReadingPositionStore(context) }
    val lifecycleOwner = LocalLifecycleOwner.current

    var session by remember { mutableStateOf<FfiReaderSession?>(null) }
    var snapshot by remember { mutableStateOf<ReaderSnapshot?>(null) }
    var html by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    var fontScale by remember { mutableFloatStateOf(1f) }
    var activeSample by remember { mutableStateOf<String?>(null) }
    var searchQuery by remember { mutableStateOf("") }
    var searchResults by remember { mutableStateOf<List<SearchResultItem>>(emptyList()) }
    var isSearchExpanded by remember { mutableStateOf(false) }

    fun saveCurrentPosition() {
        val s = session ?: return
        val snap = snapshot ?: return
        val pos = s.getPosition()
        positionStore.save(snap.bookId, pos)
    }

    fun refresh(from: FfiReaderSession, snap: ReaderSnapshot, savePosition: Boolean = true) {
        if (savePosition) saveCurrentPosition()
        session = from
        snapshot = snap
        fontScale = snap.fontScale
        html = from.currentHtml()
        error = null
    }

    fun openBytes(name: String, bytes: ByteArray, sampleId: String?) {
        runCatching {
            saveCurrentPosition()
            val opened = engine.open(name, bytes)
            var snap = opened.snapshot()
            val savedPos = positionStore.load(snap.bookId)
            if (savedPos != null) {
                snap = opened.restorePosition(savedPos)
            }
            refresh(opened, snap, savePosition = false)
            activeSample = sampleId
            searchQuery = ""
            searchResults = emptyList()
        }.onFailure {
            error = it.message ?: it.toString()
        }
    }

    fun performSearch(query: String) {
        val s = session ?: return
        searchResults = if (query.isBlank()) {
            emptyList()
        } else {
            s.search(query)
        }
    }

    fun navigateToSearchResult(result: SearchResultItem) {
        val s = session ?: return
        val snap = s.restorePosition(
            SavedPosition(
                spineIndex = result.spineIndex,
                progression = result.progression,
            )
        )
        refresh(s, snap)
        isSearchExpanded = false
    }

    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_PAUSE || event == Lifecycle.Event.ON_STOP) {
                session?.let { s ->
                    snapshot?.let { snap ->
                        val pos = s.getPosition()
                        positionStore.save(snap.bookId, pos)
                    }
                }
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
        }
    }

    val picker = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        runCatching {
            openBytes(displayName(uri), readUri(uri), sampleId = null)
        }.onFailure {
            error = it.message ?: it.toString()
        }
    }

    LaunchedEffect(samples) {
        val first = samples.firstOrNull { it.id == "local-lessons-pt" }
            ?: samples.firstOrNull()
            ?: return@LaunchedEffect
        openBytes(first.fileName, readAsset(first.assetPath), first.id)
    }

    Scaffold(
        modifier = ComposeModifier
            .fillMaxSize()
            .statusBarsPadding()
            .navigationBarsPadding(),
        bottomBar = {
            val snap = snapshot
            val index = snap?.spineIndex?.toInt() ?: 0
            val total = snap?.spineLen?.toInt() ?: 0
            Row(
                modifier = ComposeModifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                OutlinedButton(
                    onClick = { session?.let { refresh(it, it.prev()) } },
                    enabled = snap != null && index > 0,
                    modifier = ComposeModifier.height(48.dp),
                ) {
                    Text("Anterior")
                }
                Text(
                    text = if (snap == null) "-" else "${index + 1} / $total",
                    modifier = ComposeModifier.weight(1f),
                    style = MaterialTheme.typography.bodyMedium,
                )
                Button(
                    onClick = { session?.let { refresh(it, it.next()) } },
                    enabled = snap != null && index + 1 < total,
                    modifier = ComposeModifier.height(48.dp),
                ) {
                    Text("Proxima")
                }
            }
        },
    ) { padding ->
        Column(
            modifier = ComposeModifier
                .fillMaxSize()
                .padding(padding)
                .padding(horizontal = 16.dp),
        ) {
            Text(
                text = "reader-engine",
                style = MaterialTheme.typography.headlineLarge.copy(fontWeight = FontWeight.Bold),
            )
            Text(
                text = snapshot?.let { "${it.title} · ${it.authors.joinToString()}" }
                    ?: "Abra um arquivo .epub / .json ou use uma amostra",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.75f),
            )
            Spacer(modifier = ComposeModifier.height(12.dp))
            if (session != null) {
                SearchBar(
                    query = searchQuery,
                    onQueryChange = { newQuery ->
                        searchQuery = newQuery
                        performSearch(newQuery)
                    },
                    isExpanded = isSearchExpanded,
                    onExpandedChange = { isSearchExpanded = it },
                    results = searchResults,
                    onResultClick = { result -> navigateToSearchResult(result) },
                    snapshot = snapshot,
                )
            }
            Row(
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
                modifier = ComposeModifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
            ) {
                samples.forEach { sample ->
                    FilterChip(
                        selected = activeSample == sample.id,
                        onClick = {
                            openBytes(sample.fileName, readAsset(sample.assetPath), sample.id)
                        },
                        label = { Text(sample.title) },
                    )
                }
                OutlinedButton(
                    onClick = {
                        picker.launch(
                            arrayOf(
                                "application/epub+zip",
                                "application/json",
                                "text/json",
                                "text/plain",
                                "application/octet-stream",
                                "*/*",
                            ),
                        )
                    },
                    modifier = ComposeModifier.height(40.dp),
                ) {
                    Text("Abrir arquivo")
                }
            }
            Spacer(modifier = ComposeModifier.height(8.dp))
            Text("Texto", style = MaterialTheme.typography.labelMedium)
            Slider(
                value = fontScale,
                onValueChange = { fontScale = it },
                onValueChangeFinished = {
                    session?.let { refresh(it, it.setFontScale(fontScale)) }
                },
                valueRange = 0.8f..1.8f,
            )
            error?.let {
                Text(it, color = MaterialTheme.colorScheme.error)
                Spacer(modifier = ComposeModifier.height(8.dp))
            }
            AndroidView(
                modifier = ComposeModifier
                    .fillMaxWidth()
                    .weight(1f),
                factory = { context ->
                    WebView(context).apply {
                        webViewClient = BookResourceWebViewClient { session }
                        settings.javaScriptEnabled = false
                        settings.allowFileAccess = false
                        settings.allowContentAccess = false
                        setBackgroundColor(0xFFF3FAF7.toInt())
                    }
                },
                update = { webView ->
                    webView.loadDataWithBaseURL(
                        BOOK_BASE_URL,
                        html.ifBlank { "<html><body></body></html>" },
                        "text/html",
                        "utf-8",
                        null,
                    )
                },
            )
        }
    }
}

@Composable
private fun SearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
    isExpanded: Boolean,
    onExpandedChange: (Boolean) -> Unit,
    results: List<SearchResultItem>,
    onResultClick: (SearchResultItem) -> Unit,
    snapshot: ReaderSnapshot?,
) {
    Column(modifier = ComposeModifier.fillMaxWidth()) {
        OutlinedTextField(
            value = query,
            onValueChange = onQueryChange,
            modifier = ComposeModifier.fillMaxWidth(),
            placeholder = { Text("Buscar no livro…") },
            singleLine = true,
            leadingIcon = {
                Icon(
                    imageVector = Icons.Filled.Search,
                    contentDescription = "Buscar",
                )
            },
            trailingIcon = {
                if (query.isNotEmpty()) {
                    IconButton(onClick = {
                        onQueryChange("")
                        onExpandedChange(false)
                    }) {
                        Icon(
                            imageVector = Icons.Filled.Close,
                            contentDescription = "Limpar",
                        )
                    }
                }
            },
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = MaterialTheme.colorScheme.primary,
                unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.5f),
            ),
        )

        if (query.isNotEmpty() && results.isNotEmpty()) {
            Spacer(modifier = ComposeModifier.height(8.dp))
            LazyColumn(
                modifier = ComposeModifier
                    .fillMaxWidth()
                    .height(200.dp),
                verticalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                items(results) { result ->
                    SearchResultRow(
                        result = result,
                        snapshot = snapshot,
                        onClick = { onResultClick(result) },
                    )
                }
            }
        } else if (query.isNotEmpty() && results.isEmpty()) {
            Spacer(modifier = ComposeModifier.height(8.dp))
            Text(
                text = "Nenhum resultado encontrado",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
            )
        }
        Spacer(modifier = ComposeModifier.height(8.dp))
    }
}

@Composable
private fun SearchResultRow(
    result: SearchResultItem,
    snapshot: ReaderSnapshot?,
    onClick: () -> Unit,
) {
    val chapterName = snapshot?.toc
        ?.find { it.spineIndex == result.spineIndex }
        ?.title
        ?: "Página ${result.spineIndex.toInt() + 1}"

    Surface(
        modifier = ComposeModifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        color = MaterialTheme.colorScheme.surface,
        shape = MaterialTheme.shapes.small,
        tonalElevation = 1.dp,
    ) {
        Column(
            modifier = ComposeModifier.padding(12.dp),
        ) {
            Text(
                text = chapterName,
                style = MaterialTheme.typography.labelMedium,
                color = MaterialTheme.colorScheme.primary,
            )
            Text(
                text = result.snippet,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f),
                maxLines = 2,
            )
        }
    }
}
