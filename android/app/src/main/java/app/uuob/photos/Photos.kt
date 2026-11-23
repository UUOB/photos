package app.uuob.photos

import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.ui.unit.DpOffset
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.input.key.isCtrlPressed
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import app.uuob.kit.DetailSheet
import app.uuob.kit.EmptyState
import app.uuob.kit.RailIcon
import app.uuob.kit.SettingsSheet
import app.uuob.kit.Shortcuts
import app.uuob.kit.ShortcutsHost
import app.uuob.kit.SidebarRow
import app.uuob.kit.SidebarScaffold
import app.uuob.kit.LocalDrawerMode
import app.uuob.kit.Toolbar
import androidx.compose.foundation.layout.height
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.ImageDecoder
import android.graphics.Matrix
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.automirrored.rounded.KeyboardArrowLeft
import androidx.compose.material.icons.automirrored.rounded.KeyboardArrowRight
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.Delete
import androidx.compose.material.icons.rounded.Photo
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LargeTopAppBar
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.exifinterface.media.ExifInterface
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.ui.draw.shadow
import app.uuob.kit.Btn
import app.uuob.kit.BtnKind
import app.uuob.kit.Colors
import app.uuob.kit.Icon24
import app.uuob.kit.IconBtn
import app.uuob.kit.IconRow
import app.uuob.kit.Pill
import app.uuob.kit.SectionLabel
import app.uuob.kit.Tokens
import app.uuob.kit.WebCard
import app.uuob.kit.WebSheet
import app.uuob.kit.UpdateButton
import app.uuob.kit.Updater
import app.uuob.kit.Uuob
import app.uuob.kit.json
import app.uuob.kit.objects
import app.uuob.kit.userMessage
import app.uuob.photos.core.daysLeft
import app.uuob.photos.core.details
import app.uuob.photos.core.saveName
import app.uuob.photos.core.durationLabel
import app.uuob.photos.core.expired
import app.uuob.photos.core.galleryView
import app.uuob.photos.core.jpegDataUrl
import app.uuob.photos.core.markDeleted
import app.uuob.photos.core.monthContainerKey
import app.uuob.photos.core.monthOf
import app.uuob.photos.core.newRecord
import app.uuob.photos.core.stored
import app.uuob.photos.core.takenOrNow
import app.uuob.photos.core.thumbBytes
import app.uuob.photos.core.thumbSize
import app.uuob.photos.core.whenLabel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONArray
import org.json.JSONObject
import java.io.ByteArrayOutputStream
import java.io.File

/**
 * Photos' state: the months (containers) and every photo record, kept live
 * from UUOB. The rules are the core's (core/src/lib.rs); the data is UUOB's.
 */
class PhotosModel(private val ctx: Context) {
  private val client get() = Uuob.client
  var months by mutableStateOf<List<JSONObject>>(emptyList())
  var all by mutableStateOf("[]")
  var adding by mutableStateOf<Pair<Int, Int>?>(null)
  var error by mutableStateOf("")
  private val made = mutableMapOf<String, String>()
  private val asked = mutableSetOf<String>()
  private var purged = false

  suspend fun load() {
    // What was seen last, at once; then UUOB's.
    if (all == "[]") all = client.cachedAll("photos")
    try {
      months = objects(client.containers()).filter { !it.optBoolean("main") }
      // Phones keep the lists; the originals stay on the storage devices.
      for (c in months) {
        val id = c.optString("id")
        if (c.optString("keep") == "all" && asked.add(id)) runCatching { client.keep(id, "list") }
      }
    } catch (_: Exception) {}
    val result = JSONObject(client.all("photos"))
    val records = result.optJSONArray("records") ?: JSONArray()
    all = records.toString()
    if (!purged && records.length() > 0) {
      purged = true
      for (p in objects(expired(all, System.currentTimeMillis()))) destroy(p)
    }
  }

  /** The container of a month, created when its first photo is added. Its id is the same on every device. */
  private suspend fun monthContainer(key: String): String =
    months.firstOrNull { it.optString("name") == key }?.optString("id") ?: made[key]
      ?: client.createContainer(key, monthContainerKey(key)).also { made[key] = it }

  suspend fun add(uris: List<Uri>) {
    error = ""
    adding = 0 to uris.size
    for ((i, uri) in uris.withIndex()) {
      var name = "photo"
      try {
        val info = withContext(Dispatchers.IO) { describe(uri) }
        name = info.name
        val taken = takenOrNow(info.modified, System.currentTimeMillis())
        val container = monthContainer(monthOf(taken))
        // A format this device can't draw still gets stored, without a preview.
        val preview = withContext(Dispatchers.IO) { runCatching { preview(uri, info.type) }.getOrDefault(JSONObject()) }
        val copy = withContext(Dispatchers.IO) {
          File(ctx.cacheDir, "adding").also { f -> ctx.contentResolver.openInputStream(uri)!!.use { it.copyTo(f.outputStream()) } }
        }
        val file = try {
          JSONObject(client.putFilePath(container, copy.absolutePath, info.type))
        } finally {
          copy.delete()
        }
        val record = newRecord(taken, System.currentTimeMillis(), info.name, info.type, info.size, file.optString("id"), preview.toString())
        client.put("photos", container, null, record)
      } catch (e: Exception) {
        error = "$name could not be added. ${e.userMessage}"
      }
      adding = (i + 1) to uris.size
    }
    adding = null
    load()
  }

  private class Info(val name: String, val type: String, val size: Long, val modified: Long)

  private fun describe(uri: Uri): Info {
    var name = "photo"
    var size = 0L
    ctx.contentResolver.query(uri, null, null, null, null)?.use { c ->
      if (c.moveToFirst()) {
        c.getColumnIndex(OpenableColumns.DISPLAY_NAME).takeIf { it >= 0 }?.let { name = c.getString(it) ?: name }
        c.getColumnIndex(OpenableColumns.SIZE).takeIf { it >= 0 }?.let { size = c.getLong(it) }
      }
    }
    // When it was taken: the media's own date, as the web picker's lastModified did.
    var modified = 0L
    runCatching {
      ctx.contentResolver.query(uri, arrayOf("datetaken", "date_modified"), null, null, null)?.use { c ->
        if (c.moveToFirst()) {
          val taken = c.getColumnIndex("datetaken").takeIf { it >= 0 }?.let { c.getLong(it) } ?: 0L
          val mod = c.getColumnIndex("date_modified").takeIf { it >= 0 }?.let { c.getLong(it) * 1000 } ?: 0L
          modified = if (taken > 0) taken else mod
        }
      }
    }
    return Info(name, ctx.contentResolver.getType(uri) ?: "application/octet-stream", size, modified)
  }

  /** `{ w, h, thumb }` for a picture, `{ w, h, duration, thumb }` for a video (its frame at 0.1 s). */
  private fun preview(uri: Uri, type: String): JSONObject {
    val out = JSONObject()
    val full: Bitmap
    if (type.startsWith("video/")) {
      val r = MediaMetadataRetriever()
      try {
        r.setDataSource(ctx, uri)
        val ms = r.extractMetadata(MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull() ?: 0L
        full = r.getFrameAtTime(minOf(100_000L, ms * 1000), MediaMetadataRetriever.OPTION_CLOSEST) ?: return out
        out.put("duration", ms / 1000.0)
      } finally {
        r.release()
      }
    } else {
      full = decode(ctx.contentResolver.openInputStream(uri)!!.use { it.readBytes() }, 1600) ?: return out
    }
    val (w, h) = full.width to full.height
    val size = thumbSize(w.toUInt(), h.toUInt())
    val small = Bitmap.createScaledBitmap(full, size[0].toInt(), size[1].toInt(), true)
    val jpeg = ByteArrayOutputStream().also { small.compress(Bitmap.CompressFormat.JPEG, 60, it) }.toByteArray()
    // Sizes of the original, as the web version kept them (the decoded size, turned upright).
    val bounds = if (type.startsWith("video/")) w to h else uprightSize(uri) ?: (w to h)
    out.put("w", bounds.first).put("h", bounds.second).put("thumb", jpegDataUrl(jpeg))
    return out
  }

  private fun uprightSize(uri: Uri): Pair<Int, Int>? {
    val o = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    ctx.contentResolver.openInputStream(uri)?.use { BitmapFactory.decodeStream(it, null, o) }
    if (o.outWidth <= 0) return null
    val rotate = ctx.contentResolver.openInputStream(uri)?.use { ExifInterface(it).rotationDegrees } ?: 0
    return if (rotate % 180 == 90) o.outHeight to o.outWidth else o.outWidth to o.outHeight
  }

  suspend fun save(p: JSONObject) {
    client.put("photos", p.optString("container"), null, stored(p.toString()))
  }

  /** Deleting moves a photo to Recently Deleted; it is gone for good later. */
  suspend fun remove(p: JSONObject) = client.put("photos", p.optString("container"), null, markDeleted(p.toString(), System.currentTimeMillis()))
  suspend fun restore(p: JSONObject) = client.put("photos", p.optString("container"), null, markDeleted(p.toString(), 0))

  /**
   * Saves the originals to this phone's gallery (Pictures/UUOB, videos in
   * Movies/UUOB). Returns how many were saved; the rest were on a storage
   * device that couldn't be reached.
   */
  suspend fun download(list: List<JSONObject>): Int = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
    var saved = 0
    for (p in list) {
      val bytes = runCatching { client.readFile(p.optString("container"), p.optString("file")) }.getOrNull() ?: continue
      val video = isVideo(p)
      val type = p.optString("type").ifEmpty { if (video) "video/mp4" else "image/jpeg" }
      val name = p.optString("name").ifEmpty { "UUOB-${p.optLong("taken")}" }
      runCatching {
        if (android.os.Build.VERSION.SDK_INT >= 29) {
          val values = android.content.ContentValues().apply {
            put(android.provider.MediaStore.MediaColumns.DISPLAY_NAME, name)
            put(android.provider.MediaStore.MediaColumns.MIME_TYPE, type)
            put(android.provider.MediaStore.MediaColumns.RELATIVE_PATH, (if (video) "Movies" else "Pictures") + "/UUOB")
            if (p.has("taken")) put(android.provider.MediaStore.MediaColumns.DATE_TAKEN, p.optLong("taken"))
          }
          val where = if (video) android.provider.MediaStore.Video.Media.EXTERNAL_CONTENT_URI else android.provider.MediaStore.Images.Media.EXTERNAL_CONTENT_URI
          val uri = ctx.contentResolver.insert(where, values)!!
          ctx.contentResolver.openOutputStream(uri)!!.use { it.write(bytes) }
        } else {
          // Older Android: the app's own Pictures folder (no storage permission needed).
          val dir = ctx.getExternalFilesDir(if (video) android.os.Environment.DIRECTORY_MOVIES else android.os.Environment.DIRECTORY_PICTURES)!!
          File(dir, name).writeBytes(bytes)
        }
        saved++
      }
    }
    saved
  }

  /** Hands the originals to another app (the phone's share sheet). False when none could be read. */
  suspend fun share(list: List<JSONObject>): Boolean {
    val uris = kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
      val dir = File(ctx.cacheDir, "shared").apply { deleteRecursively(); mkdirs() }
      list.mapNotNull { p ->
        val bytes = runCatching { client.readFile(p.optString("container"), p.optString("file")) }.getOrNull() ?: return@mapNotNull null
        val f = File(dir, saveName(p.toString())).apply { writeBytes(bytes) }
        androidx.core.content.FileProvider.getUriForFile(ctx, ctx.packageName + ".files", f)
      }
    }
    if (uris.isEmpty()) return false
    val type = if (list.all { isVideo(it) }) "video/*" else if (list.none { isVideo(it) }) "image/*" else "*/*"
    val send = if (uris.size == 1) android.content.Intent(android.content.Intent.ACTION_SEND).putExtra(android.content.Intent.EXTRA_STREAM, uris[0])
      else android.content.Intent(android.content.Intent.ACTION_SEND_MULTIPLE).putParcelableArrayListExtra(android.content.Intent.EXTRA_STREAM, ArrayList(uris))
    send.setType(type).addFlags(android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION)
    ctx.startActivity(android.content.Intent.createChooser(send, null).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK))
    return true
  }

  suspend fun destroy(p: JSONObject) {
    val c = p.optString("container")
    runCatching { client.deleteFile(c, p.optString("file")) }
    runCatching { client.delete("photos", c, p.optString("id")) }
  }
}

/** Decodes a picture upright, no larger than about `max` on its long side. */
fun decode(bytes: ByteArray, max: Int): Bitmap? {
  if (Build.VERSION.SDK_INT >= 28) {
    return runCatching {
      ImageDecoder.decodeBitmap(ImageDecoder.createSource(java.nio.ByteBuffer.wrap(bytes))) { d, info, _ ->
        val s = maxOf(info.size.width, info.size.height)
        if (s > max) d.setTargetSize(info.size.width * max / s, info.size.height * max / s)
        d.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
      }
    }.getOrNull()
  }
  val o = BitmapFactory.Options().apply { inJustDecodeBounds = true }
  BitmapFactory.decodeByteArray(bytes, 0, bytes.size, o)
  var sample = 1
  while (maxOf(o.outWidth, o.outHeight) / (sample * 2) >= max) sample *= 2
  val b = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, BitmapFactory.Options().apply { inSampleSize = sample }) ?: return null
  val rotate = runCatching { ExifInterface(bytes.inputStream()).rotationDegrees }.getOrDefault(0)
  return if (rotate == 0) b else Bitmap.createBitmap(b, 0, 0, b.width, b.height, Matrix().apply { postRotate(rotate.toFloat()) }, true)
}

private val thumbs = mutableMapOf<String, ImageBitmap?>()

/** A record's preview, decoded once. */
fun thumbOf(p: JSONObject): ImageBitmap? {
  val t = p.optString("thumb").ifEmpty { return null }
  val key = p.optString("container") + "/" + p.optString("id") + "/" + t.length
  return thumbs.getOrPut(key) {
    val b = thumbBytes(t)
    BitmapFactory.decodeByteArray(b, 0, b.size)?.asImageBitmap()
  }
}

fun isVideo(p: JSONObject) = p.optString("type").startsWith("video/")

@Composable
fun PhotosScreen(model: PhotosModel, updater: Updater) {
  val scope = rememberCoroutineScope()
  // The place shown: "all", "videos" or "trash" (Recently deleted); the phone's drawer; Settings.
  var view by remember { mutableStateOf("all") }
  var drawerOpen by remember { mutableStateOf(false) }
  var settingsOpen by remember { mutableStateOf(false) }
  val trashView = view == "trash"
  var open by remember { mutableStateOf<JSONObject?>(null) }
  var confirm by remember { mutableStateOf<JSONObject?>(null) }
  // Long-press a photo to choose several: then Download or Delete them together.
  var chosen by remember { mutableStateOf(setOf<String>()) }
  var note by remember { mutableStateOf("") }
  fun keyOf(p: JSONObject) = p.optString("container") + "/" + p.optString("id")
  fun download(list: List<JSONObject>) = scope.launch {
    note = "Downloading…"
    val n = model.download(list)
    note = if (n == list.size) "Saved to your gallery" else "Saved $n of ${list.size}: the rest are on a device that can’t be reached"
  }
  fun share(list: List<JSONObject>) = scope.launch {
    if (!model.share(list)) note = "The original is on a storage device that can’t be reached right now."
  }
  var info by remember { mutableStateOf<JSONObject?>(null) }
  val picker = rememberLauncherForActivityResult(ActivityResultContracts.PickMultipleVisualMedia()) { uris ->
    if (uris.isNotEmpty()) scope.launch { model.add(uris) }
  }
  fun pick() = picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageAndVideo))

  LaunchedEffect(Unit) {
    model.load()
    Uuob.events.collect { e -> if (e.optString("type") == "changed" || (e.optString("type") == "state" && e.optString("state") == "connected")) model.load() }
  }
  val g = remember(model.all, view) { JSONObject(galleryView(model.all, view)) }
  val photoCount = g.optInt("photos")
  val trashCount = g.optInt("trash")
  // The sidebar's places: (id, name, icon, count); Recently deleted only while something is there.
  val places = listOf(Triple("all", "Photos", "photo") to photoCount, Triple("videos", "Videos", "play") to g.optInt("videos"), Triple("trash", "Recently deleted", "trash") to trashCount)
    .filter { (p, n) -> p.first != "trash" || n > 0 || trashView }
  fun go(id: String) { view = id; chosen = emptySet(); drawerOpen = false }
  val groups = objects(g.optJSONArray("groups")?.toString())
  val shown = groups.flatMap { objects(it.optJSONArray("items")?.toString()) }
  BackHandler(enabled = view != "all" && open == null && chosen.isEmpty()) { view = "all" }
  BackHandler(enabled = chosen.isNotEmpty() && open == null) { chosen = emptySet() }
  val picked = shown.filter { keyOf(it) in chosen }

  val shortcuts = Shortcuts(
    settings = { settingsOpen = true },
    new = "Add photos" to { pick() },
    refresh = { scope.launch { model.load() } },
    extra = places.mapIndexed { i, (p, _) -> Shortcuts.Extra(p.second, "Ctrl+${i + 1}", listOf(Key.One, Key.Two, Key.Three)[i], true) { go(p.first) } },
  )
  ShortcutsHost(shortcuts) {
  SidebarScaffold(drawerOpen, { drawerOpen = it }, sidebar = {
    Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
      places.forEach { (p, n) ->
        SidebarRow(p.second, view == p.first, { go(p.first) }, leading = { Icon24(p.third, size = 19.dp, tint = Tokens.text2) }) { if (n > 0 && p.first != "trash") Text("$n", fontSize = 14.sp) }
      }
    }
  }, rail = {
    Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
      places.forEach { (p, _) -> RailIcon(p.third, p.second, view == p.first) { go(p.first) } }
    }
    RailIcon("settings", "Settings") { settingsOpen = true }
  }, footer = {
    SidebarRow("Settings", onClick = { drawerOpen = false; settingsOpen = true }, leading = { Icon24("settings", size = 19.dp, tint = Tokens.text2) })
  }) {
  BoxWithConstraints(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background)) {
    val phone = LocalDrawerMode.current
    val compactTitle = maxWidth < 720.dp
    LazyVerticalGrid(
      columns = if (phone) GridCells.Fixed(3) else GridCells.Adaptive(150.dp),
      contentPadding = PaddingValues(start = if (phone) 14.dp else 28.dp, end = if (phone) 14.dp else 28.dp, top = if (phone) 12.dp else 26.dp, bottom = 48.dp),
      horizontalArrangement = Arrangement.spacedBy(if (phone) 3.dp else 4.dp),
      verticalArrangement = Arrangement.spacedBy(if (phone) 3.dp else 4.dp),
      modifier = Modifier.fillMaxSize().safeDrawingPadding().widthIn(max = 1104.dp).align(Alignment.TopCenter),
    ) {
      item(span = { GridItemSpan(maxLineSpan) }) {
        Row(Modifier.padding(bottom = if (chosen.isNotEmpty()) 18.dp else 0.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
          if (chosen.isNotEmpty()) {
            IconBtn("close", { chosen = emptySet() }, description = "Clear selection")
            Text("${chosen.size} selected", fontSize = if (phone) 26.sp else 30.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.6).sp, maxLines = 1, modifier = Modifier.weight(1f))
            if (!trashView) {
              IconBtn("share", { share(picked); chosen = emptySet() }, size = 20.dp, description = "Share")
              IconBtn("download", { download(picked); chosen = emptySet() }, size = 20.dp, description = "Download")
              IconBtn("trash", { val list = picked; chosen = emptySet(); scope.launch { list.forEach { model.remove(it) } } }, size = 20.dp, description = "Delete")
            } else {
              Btn("Restore", { val list = picked; chosen = emptySet(); scope.launch { list.forEach { model.restore(it) } } })
            }
          } else {
            // The toolbar: the place and its count in grey, the update, the +.
            val here = places.firstOrNull { it.first.first == view } ?: places.first()
            val title = buildAnnotatedString {
              withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append(here.first.second) }
              if (here.second > 0) withStyle(SpanStyle(fontWeight = FontWeight.Normal, color = Tokens.text3)) { append(" ${here.second}") }
            }
            Box(Modifier.weight(1f)) {
              Toolbar(title, compact = compactTitle, menu = if (phone) ({ drawerOpen = true }) else null, add = if (!trashView && model.adding == null) ::pick else null, addDescription = "Add photos") { UpdateButton(updater) }
            }
          }
        }
      }
      if (model.error.isNotEmpty()) item(span = { GridItemSpan(maxLineSpan) }) {
        Text(model.error, Modifier.padding(bottom = 14.dp), color = MaterialTheme.colorScheme.error)
      }
      if (groups.isEmpty()) item(span = { GridItemSpan(maxLineSpan) }) {
        when (view) {
          "trash" -> EmptyState("trash", "Nothing here")
          "videos" -> EmptyState("play", "No videos")
          else -> EmptyState("photo", "No photos yet", color = Colors.of("orange"), action = if (model.adding == null) "Add photos" to { pick() } else null)
        }
      }
      groups.forEachIndexed { gi, group ->
        item(span = { GridItemSpan(maxLineSpan) }, key = "m" + group.optString("month")) {
          SectionLabel(group.optString("label"), Modifier.padding(top = if (gi == 0) 6.dp else 26.dp, bottom = 10.dp))
        }
        items(objects(group.optJSONArray("items")?.toString()), key = { it.optString("container") + "/" + it.optString("id") }) { p ->
          val k = keyOf(p)
          Cell(p, trashView, if (phone) 3.dp else 6.dp, selecting = chosen.isNotEmpty(), selected = k in chosen,
            onLongClick = { chosen = chosen + k },
          ) { if (chosen.isNotEmpty()) chosen = if (k in chosen) chosen - k else chosen + k else open = p }
        }
      }
    }
    if (note.isNotEmpty()) {
      Text(
        note, fontSize = 14.sp, textAlign = TextAlign.Center,
        modifier = Modifier.align(Alignment.BottomCenter).safeDrawingPadding().padding(20.dp).shadow(8.dp, Pill.shape).clip(Pill.shape)
          .background(MaterialTheme.colorScheme.surface).border(1.dp, Tokens.line, Pill.shape).clickable { note = "" }.padding(horizontal = 18.dp, vertical = 10.dp),
      )
      LaunchedEffect(note) { if (note != "Downloading…") { kotlinx.coroutines.delay(3500); note = "" } }
    }
    model.adding?.let { (done, total) ->
      Text(
        "Adding ${minOf(done + 1, total)} of $total…", fontSize = 14.sp,
        modifier = Modifier.align(Alignment.BottomCenter).safeDrawingPadding().padding(20.dp).shadow(8.dp, Pill.shape).clip(Pill.shape)
          .background(MaterialTheme.colorScheme.surface).border(1.dp, Tokens.line, Pill.shape).padding(horizontal = 18.dp, vertical = 10.dp),
      )
    }
  }
  }
  }
  if (settingsOpen) SettingsSheet(onClose = { settingsOpen = false }) {}
  open?.let { p ->
    Viewer(
      p, shown,
      onShow = { open = it },
      onClose = { open = null },
      onDelete = {
        val i = shown.indexOfFirst { it.optString("id") == p.optString("id") }
        open = shown.getOrNull(i + 1)
        scope.launch { model.remove(p) }
      },
      onRestore = { open = null; scope.launch { model.restore(p) } },
      onDestroy = { confirm = p },
      onDownload = { download(listOf(p)) },
      onShare = { share(listOf(p)) },
      onInfo = { info = p },
    )
  }
  // A photo's details, in the event card's style.
  info?.let { p ->
    DetailSheet("Info", { info = null }) {
      Spacer(Modifier.height(6.dp))
      val rows = objects(details(p.toString()))
      rows.forEachIndexed { i, r ->
        IconRow(r.optString("icon"), last = i == rows.size - 1) {
          Text(r.optString("label"), fontSize = 15.sp, color = Tokens.text2)
          Spacer(Modifier.weight(1f))
          Text(r.optString("value"), fontSize = 15.sp, maxLines = 1, overflow = androidx.compose.ui.text.style.TextOverflow.MiddleEllipsis)
        }
      }
    }
  }
  confirm?.let { p ->
    val what = if (isVideo(p)) "video" else "photo"
    WebSheet({ confirm = null }) {
      Text("Delete this $what?", fontSize = 22.sp, fontWeight = FontWeight.SemiBold, letterSpacing = (-0.4).sp)
      Text("It is removed from all your devices. This can’t be undone.", color = Tokens.text2, modifier = Modifier.padding(top = 10.dp))
      Row(Modifier.fillMaxWidth().padding(top = 22.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Btn("Cancel", { confirm = null }, Modifier.weight(1f), lg = true)
        Btn("Delete", { confirm = null; open = null; scope.launch { model.destroy(p); model.load() } }, Modifier.weight(1f), kind = BtnKind.Danger, lg = true)
      }
    }
  }
}

@Composable
@OptIn(androidx.compose.foundation.ExperimentalFoundationApi::class)
private fun Cell(
  p: JSONObject, trash: Boolean, radius: androidx.compose.ui.unit.Dp,
  selecting: Boolean = false, selected: Boolean = false, onLongClick: () -> Unit = {}, onClick: () -> Unit,
) {
  val haptic = androidx.compose.ui.platform.LocalHapticFeedback.current
  Box(Modifier.aspectRatio(1f).clip(RoundedCornerShape(radius)).background(Tokens.surface2)
    .combinedClickable(onLongClick = { haptic.performHapticFeedback(androidx.compose.ui.hapticfeedback.HapticFeedbackType.LongPress); onLongClick() }, onClick = onClick)) {
    val thumb = thumbOf(p)
    if (thumb != null) Image(thumb, if (isVideo(p)) "Video from ${whenLabel(p.optLong("taken"))}" else "Photo from ${whenLabel(p.optLong("taken"))}", Modifier.fillMaxSize(), contentScale = ContentScale.Crop)
    else Icon24(if (isVideo(p)) "play" else "photo", Modifier.align(Alignment.Center), size = 28.dp, tint = Tokens.text3)
    val badge = when {
      trash -> daysLeft(p.optLong("deleted"), System.currentTimeMillis()).let { "$it ${if (it == 1L) "day" else "days"}" }
      isVideo(p) -> if (p.has("duration")) durationLabel(p.optDouble("duration")) else ""
      else -> null
    }
    if (badge != null) {
      Row(
        Modifier.align(Alignment.BottomEnd).padding(6.dp).clip(Pill.shape).background(Color.Black.copy(alpha = 0.55f)).padding(horizontal = 7.dp, vertical = 2.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(4.dp),
      ) {
        if (!trash) Icon24("play", size = 11.dp, stroke = 2.4f, tint = Color.White)
        Text(badge, color = Color.White, fontSize = 12.sp, fontWeight = FontWeight.SemiBold)
      }
    }
    if (selecting) {
      if (selected) Box(Modifier.fillMaxSize().background(Color.White.copy(alpha = 0.25f)))
      Box(
        Modifier.align(Alignment.TopEnd).padding(6.dp).size(22.dp).clip(CircleShape)
          .background(if (selected) MaterialTheme.colorScheme.primary else Color.Black.copy(alpha = 0.25f))
          .border(1.5.dp, Color.White, CircleShape),
        contentAlignment = Alignment.Center,
      ) { if (selected) Icon24("check", size = 14.dp, stroke = 2.6f, tint = Color.White) }
    }
  }
}

/** `.viewer`: one photo or video on black, the preview at once, then the original. */
@Composable
private fun Viewer(
  p: JSONObject, shown: List<JSONObject>,
  onShow: (JSONObject) -> Unit, onClose: () -> Unit, onDelete: () -> Unit, onRestore: () -> Unit, onDestroy: () -> Unit,
  onDownload: () -> Unit,
  onShare: () -> Unit,
  onInfo: () -> Unit,
) {
  val i = shown.indexOfFirst { it.optString("id") == p.optString("id") && it.optString("container") == p.optString("container") }
  fun step(by: Int) = shown.getOrNull(i + by)?.let(onShow)
  val focus = remember { FocusRequester() }
  BackHandler(onBack = onClose)
  LaunchedEffect(Unit) { focus.requestFocus() }
  BoxWithConstraints(
    Modifier.fillMaxSize().background(Color.Black)
      .focusRequester(focus).focusable()
      .onKeyEvent {
        if (it.type != KeyEventType.KeyDown) return@onKeyEvent false
        when (it.key) {
          Key.Escape, Key.Spacebar -> { onClose(); true }
          Key.DirectionRight -> { step(1); true }
          Key.DirectionLeft -> { step(-1); true }
          Key.S -> if (it.isCtrlPressed && !p.has("deleted")) { onDownload(); true } else false
          Key.I -> if (!p.has("deleted")) { onInfo(); true } else false
          Key.Delete, Key.Backspace -> if (!p.has("deleted")) { onDelete(); true } else false
          else -> false
        }
      },
  ) {
    val phone = maxWidth < 720.dp
    Column(Modifier.fillMaxSize()) {
      Row(Modifier.fillMaxWidth().statusBarsPadding().padding(horizontal = 12.dp, vertical = 10.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        IconBtn("close", onClose, tint = Color.White, description = "Close")
        Text(whenLabel(p.optLong("taken")), color = Color.White.copy(alpha = 0.8f), fontSize = 15.sp, modifier = Modifier.weight(1f))
        if (p.has("deleted")) {
          Btn("Restore", onRestore)
          Btn("Delete", onDestroy, kind = BtnKind.Danger)
        } else {
          IconBtn("info", onInfo, size = 20.dp, tint = Color.White, description = "Info")
          IconBtn("share", onShare, size = 20.dp, tint = Color.White, description = "Share")
          IconBtn("download", onDownload, size = 20.dp, tint = Color.White, description = "Download")
          IconBtn("trash", onDelete, size = 20.dp, tint = Color.White, description = "Delete")
        }
      }
      // A long press: what the bar offers, as a menu by the finger.
      var menu by remember(p.optString("id")) { mutableStateOf<DpOffset?>(null) }
      val density = LocalDensity.current
      Box(Modifier.weight(1f).fillMaxWidth()) {
        Original(p, Modifier.fillMaxSize().navigationBarsPadding().pointerInput(p.optString("id")) {
          if (isVideo(p)) return@pointerInput
          var dx = 0f
          detectHorizontalDragGestures(onDragStart = { dx = 0f }, onDragEnd = { if (dx < -60) step(1) else if (dx > 60) step(-1) }) { _, d -> dx += d }
        }.pointerInput(p.optString("id")) {
          detectTapGestures(onLongPress = { at -> menu = with(density) { DpOffset(at.x.toDp(), at.y.toDp()) } })
        })
        Box(Modifier.offset(menu?.x ?: 0.dp, menu?.y ?: 0.dp)) {
          DropdownMenu(menu != null, { menu = null }, shape = RoundedCornerShape(14.dp), containerColor = Tokens.surface2) {
            val items = if (p.has("deleted")) listOf("Restore" to onRestore, "Delete forever…" to onDestroy)
              else listOf("Download" to onDownload, "Share" to onShare, "Info" to onInfo, "Delete" to onDelete)
            items.forEach { (label, act) -> DropdownMenuItem({ Text(label, fontSize = 15.sp, fontWeight = FontWeight.Medium) }, { menu = null; act() }) }
          }
        }
      }
    }
    // On a phone you swipe; the arrows would cover the photo.
    if (!phone) {
      if (i > 0) Box(Modifier.align(Alignment.CenterStart).padding(14.dp).size(44.dp).clip(CircleShape).background(Color.Black.copy(alpha = 0.4f)).clickable { step(-1) }, contentAlignment = Alignment.Center) { Icon24("left", tint = Color.White) }
      if (i < shown.size - 1) Box(Modifier.align(Alignment.CenterEnd).padding(14.dp).size(44.dp).clip(CircleShape).background(Color.Black.copy(alpha = 0.4f)).clickable { step(1) }, contentAlignment = Alignment.Center) { Icon24("right", tint = Color.White) }
    }
  }
}

@Composable
private fun Original(p: JSONObject, modifier: Modifier) {
  val ctx = LocalContext.current
  val key = p.optString("container") + "/" + p.optString("id")
  var image by remember(key) { mutableStateOf<ImageBitmap?>(null) }
  var missing by remember(key) { mutableStateOf(false) }
  var url by remember(key) { mutableStateOf<String?>(null) }
  LaunchedEffect(key) {
    if (isVideo(p)) {
      url = Uuob.client.fileUrl(p.optString("container"), p.optString("file"))
    } else {
      try {
        val bytes = Uuob.client.readFile(p.optString("container"), p.optString("file"))
        image = withContext(Dispatchers.Default) { decode(bytes, 4096)?.asImageBitmap() } ?: run { missing = true; null }
      } catch (_: Exception) {
        missing = true
      }
    }
  }
  Box(modifier, contentAlignment = Alignment.Center) {
    val thumb = thumbOf(p)
    when {
      missing -> {
        if (thumb != null) Image(thumb, null, Modifier.fillMaxSize(), contentScale = ContentScale.Fit, alpha = 0.5f)
        Surface(shape = RoundedCornerShape(16.dp), color = Color.Black.copy(alpha = 0.6f)) {
          Text("The original is on a storage device that can’t be reached right now.", Modifier.padding(16.dp), color = Color.White, textAlign = TextAlign.Center)
        }
      }
      isVideo(p) && url != null -> AndroidView(
        factory = { c ->
          android.widget.VideoView(c).apply {
            val controls = android.widget.MediaController(c)
            controls.setAnchorView(this)
            setMediaController(controls)
            setOnErrorListener { _, _, _ -> missing = true; true }
            setOnPreparedListener { start() }
            setVideoURI(Uri.parse(url))
          }
        },
        modifier = Modifier.fillMaxSize(),
      )
      image != null -> Image(image!!, p.optString("name"), Modifier.fillMaxSize(), contentScale = ContentScale.Fit)
      thumb != null -> Image(thumb, null, Modifier.fillMaxSize(), contentScale = ContentScale.Fit)
    }
  }
}
