use std::path::Path;

use jni::objects::{JObject, JValue};
use jni::{Env, JavaVM, jni_sig, jni_str};
use winit::platform::android::activity::AndroidApp;

pub enum Kept {
    Done,
    Asked,
    Failed,
}

type Call<T> = jni::errors::Result<T>;

fn text<'local>(env: &mut Env<'local>, value: &str) -> Call<JObject<'local>> {
    Ok(env.new_string(value)?.into())
}

// Before Android 10 a picture goes into the gallery only with this permission, which the owner grants in a
// dialog; from Android 10 on an app may add its own pictures without asking.
fn permitted(env: &mut Env<'_>, activity: &JObject<'_>) -> Call<bool> {
    if AndroidApp::sdk_version() >= 29 {
        return Ok(true);
    }
    let permission = text(env, "android.permission.WRITE_EXTERNAL_STORAGE")?;
    let state = env
        .call_method(
            activity,
            jni_str!("checkSelfPermission"),
            jni_sig!("(Ljava/lang/String;)I"),
            &[JValue::Object(&permission)],
        )?
        .i()?;
    Ok(state == 0)
}

fn allowed(env: &mut Env<'_>, activity: &JObject<'_>) -> Call<bool> {
    if permitted(env, activity)? {
        return Ok(true);
    }
    let permission = text(env, "android.permission.WRITE_EXTERNAL_STORAGE")?;
    let wanted = env.new_object_array(1, jni_str!("java/lang/String"), &permission)?;
    env.call_method(
        activity,
        jni_str!("requestPermissions"),
        jni_sig!("([Ljava/lang/String;I)V"),
        &[JValue::Object(wanted.as_ref()), JValue::Int(1)],
    )?;
    Ok(false)
}

fn into_gallery<'local>(
    env: &mut Env<'local>,
    activity: &JObject<'_>,
    name: &str,
    png: &[u8],
) -> Call<JObject<'local>> {
    let values = env.new_object(jni_str!("android/content/ContentValues"), jni_sig!("()V"), &[])?;
    let mut fields = vec![("_display_name", name), ("mime_type", "image/png")];
    if AndroidApp::sdk_version() >= 29 {
        fields.push(("relative_path", "Pictures/Fractique"));
    }
    for (key, value) in fields {
        let (key, value) = (text(env, key)?, text(env, value)?);
        env.call_method(
            &values,
            jni_str!("put"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
            &[JValue::Object(&key), JValue::Object(&value)],
        )?;
    }
    let resolver = env
        .call_method(
            activity,
            jni_str!("getContentResolver"),
            jni_sig!("()Landroid/content/ContentResolver;"),
            &[],
        )?
        .l()?;
    let pictures = env
        .get_static_field(
            jni_str!("android/provider/MediaStore$Images$Media"),
            jni_str!("EXTERNAL_CONTENT_URI"),
            jni_sig!("Landroid/net/Uri;"),
        )?
        .l()?;
    let place = env
        .call_method(
            &resolver,
            jni_str!("insert"),
            jni_sig!("(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;"),
            &[JValue::Object(&pictures), JValue::Object(&values)],
        )?
        .l()?;
    if place.is_null() {
        return Err(jni::errors::Error::NullPtr("the gallery gave no place for the picture"));
    }
    let out = env
        .call_method(
            &resolver,
            jni_str!("openOutputStream"),
            jni_sig!("(Landroid/net/Uri;)Ljava/io/OutputStream;"),
            &[JValue::Object(&place)],
        )?
        .l()?;
    let bytes = env.byte_array_from_slice(png)?;
    env.call_method(
        &out,
        jni_str!("write"),
        jni_sig!("([B)V"),
        &[JValue::Object(bytes.as_ref())],
    )?;
    env.call_method(&out, jni_str!("close"), jni_sig!("()V"), &[])?;
    Ok(place)
}

fn offer(env: &mut Env<'_>, activity: &JObject<'_>, picture: &JObject<'_>) -> Call<()> {
    let send = text(env, "android.intent.action.SEND")?;
    let intent = env.new_object(
        jni_str!("android/content/Intent"),
        jni_sig!("(Ljava/lang/String;)V"),
        &[JValue::Object(&send)],
    )?;
    let kind = text(env, "image/png")?;
    env.call_method(
        &intent,
        jni_str!("setType"),
        jni_sig!("(Ljava/lang/String;)Landroid/content/Intent;"),
        &[JValue::Object(&kind)],
    )?;
    let stream = text(env, "android.intent.extra.STREAM")?;
    env.call_method(
        &intent,
        jni_str!("putExtra"),
        jni_sig!("(Ljava/lang/String;Landroid/os/Parcelable;)Landroid/content/Intent;"),
        &[JValue::Object(&stream), JValue::Object(picture)],
    )?;
    // FLAG_GRANT_READ_URI_PERMISSION: the app that receives the picture may read it.
    env.call_method(
        &intent,
        jni_str!("addFlags"),
        jni_sig!("(I)Landroid/content/Intent;"),
        &[JValue::Int(1)],
    )?;
    let chooser = env
        .call_static_method(
            jni_str!("android/content/Intent"),
            jni_str!("createChooser"),
            jni_sig!("(Landroid/content/Intent;Ljava/lang/CharSequence;)Landroid/content/Intent;"),
            &[JValue::Object(&intent), JValue::Object(&JObject::null())],
        )?
        .l()?;
    env.call_method(
        activity,
        jni_str!("startActivity"),
        jni_sig!("(Landroid/content/Intent;)V"),
        &[JValue::Object(&chooser)],
    )?;
    Ok(())
}

fn system_insets(env: &mut Env<'_>, activity: &JObject<'_>) -> Call<[i32; 4]> {
    let window = env
        .call_method(
            activity,
            jni_str!("getWindow"),
            jni_sig!("()Landroid/view/Window;"),
            &[],
        )?
        .l()?;
    let view = env
        .call_method(
            &window,
            jni_str!("getDecorView"),
            jni_sig!("()Landroid/view/View;"),
            &[],
        )?
        .l()?;
    let insets = env
        .call_method(
            &view,
            jni_str!("getRootWindowInsets"),
            jni_sig!("()Landroid/view/WindowInsets;"),
            &[],
        )?
        .l()?;
    if insets.is_null() {
        return Ok([0; 4]);
    }
    let mut sides = [
        env.call_method(&insets, jni_str!("getSystemWindowInsetLeft"), jni_sig!("()I"), &[])?
            .i()?,
        env.call_method(&insets, jni_str!("getSystemWindowInsetTop"), jni_sig!("()I"), &[])?
            .i()?,
        env.call_method(&insets, jni_str!("getSystemWindowInsetRight"), jni_sig!("()I"), &[])?
            .i()?,
        env.call_method(&insets, jni_str!("getSystemWindowInsetBottom"), jni_sig!("()I"), &[])?
            .i()?,
    ];
    if AndroidApp::sdk_version() >= 28 {
        let cutout = env
            .call_method(
                &insets,
                jni_str!("getDisplayCutout"),
                jni_sig!("()Landroid/view/DisplayCutout;"),
                &[],
            )?
            .l()?;
        if !cutout.is_null() {
            let safe = [
                env.call_method(&cutout, jni_str!("getSafeInsetLeft"), jni_sig!("()I"), &[])?
                    .i()?,
                env.call_method(&cutout, jni_str!("getSafeInsetTop"), jni_sig!("()I"), &[])?
                    .i()?,
                env.call_method(&cutout, jni_str!("getSafeInsetRight"), jni_sig!("()I"), &[])?
                    .i()?,
                env.call_method(&cutout, jni_str!("getSafeInsetBottom"), jni_sig!("()I"), &[])?
                    .i()?,
            ];
            for (side, safe) in sides.iter_mut().zip(safe) {
                *side = (*side).max(safe);
            }
        }
    }
    Ok(sides)
}

// The vibrator, for 50 ms, as a notification's: on the phones tried (a ZTE Blade A7 with Android 9) a pulse of any
// other use, and the view's haptic feedback, are accepted and then dropped by the system.
fn haptic(env: &mut Env<'_>, activity: &JObject<'_>) -> Call<bool> {
    const PULSE: i64 = 50;
    let name = text(env, "vibrator")?;
    let vibrator = env
        .call_method(
            activity,
            jni_str!("getSystemService"),
            jni_sig!("(Ljava/lang/String;)Ljava/lang/Object;"),
            &[JValue::Object(&name)],
        )?
        .l()?;
    if vibrator.is_null() {
        return Ok(false);
    }
    let builder = env.new_object(jni_str!("android/media/AudioAttributes$Builder"), jni_sig!("()V"), &[])?;
    env.call_method(
        &builder,
        jni_str!("setUsage"),
        jni_sig!("(I)Landroid/media/AudioAttributes$Builder;"),
        &[JValue::Int(5)],
    )?;
    let notification = env
        .call_method(
            &builder,
            jni_str!("build"),
            jni_sig!("()Landroid/media/AudioAttributes;"),
            &[],
        )?
        .l()?;
    if AndroidApp::sdk_version() >= 26 {
        let effect = env
            .call_static_method(
                jni_str!("android/os/VibrationEffect"),
                jni_str!("createOneShot"),
                jni_sig!("(JI)Landroid/os/VibrationEffect;"),
                &[JValue::Long(PULSE), JValue::Int(-1)],
            )?
            .l()?;
        env.call_method(
            &vibrator,
            jni_str!("vibrate"),
            jni_sig!("(Landroid/os/VibrationEffect;Landroid/media/AudioAttributes;)V"),
            &[JValue::Object(&effect), JValue::Object(&notification)],
        )?;
    } else {
        env.call_method(
            &vibrator,
            jni_str!("vibrate"),
            jni_sig!("(JLandroid/media/AudioAttributes;)V"),
            &[JValue::Long(PULSE), JValue::Object(&notification)],
        )?;
    }
    Ok(true)
}

pub fn refuse() {
    let Some(app) = crate::ANDROID.get() else {
        return;
    };
    // SAFETY: as in `keep`.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let done: Call<bool> = vm.attach_current_thread(|env| {
        // SAFETY: as in `keep`.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let done = haptic(env, &activity);
        if done.is_err() {
            env.exception_clear();
        }
        done
    });
    if let Err(error) = done {
        crate::note(&format!("tick: {error}"));
    }
}

pub fn insets() -> [i32; 4] {
    let Some(app) = crate::ANDROID.get() else {
        return [0; 4];
    };
    // SAFETY: as in `keep`.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let sides: Call<[i32; 4]> = vm.attach_current_thread(|env| {
        // SAFETY: as in `keep`.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let sides = system_insets(env, &activity);
        if sides.is_err() {
            env.exception_clear();
        }
        sides
    });
    sides.unwrap_or([0; 4])
}

pub fn ask() {
    let Some(app) = crate::ANDROID.get() else {
        return;
    };
    // SAFETY: as in `keep`.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let asked: Call<bool> = vm.attach_current_thread(|env| {
        // SAFETY: as in `keep`.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let asked = allowed(env, &activity);
        if asked.is_err() {
            env.exception_clear();
        }
        asked
    });
    let _ = asked;
}

pub fn granted() -> bool {
    let Some(app) = crate::ANDROID.get() else {
        return false;
    };
    // SAFETY: as in `keep`.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let granted: Call<bool> = vm.attach_current_thread(|env| {
        // SAFETY: as in `keep`.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let granted = permitted(env, &activity);
        if granted.is_err() {
            env.exception_clear();
        }
        granted
    });
    granted.unwrap_or(false)
}

pub fn keep(path: &Path, share: bool) -> Kept {
    let (Some(app), Ok(png)) = (crate::ANDROID.get(), std::fs::read(path)) else {
        return Kept::Failed;
    };
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    // SAFETY: the activity gives the pointer of the Java machine it runs in, which lives as long as the app.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) };
    let done: Call<Kept> = vm.attach_current_thread(|env| {
        // SAFETY: the pointer is the activity's own object, which outlives this call.
        let activity = unsafe { JObject::from_raw(env, app.activity_as_ptr().cast()) };
        let run = |env: &mut Env<'_>| -> Call<Kept> {
            if !allowed(env, &activity)? {
                return Ok(Kept::Asked);
            }
            let picture = into_gallery(env, &activity, &name, &png)?;
            if share {
                offer(env, &activity, &picture)?;
            }
            Ok(Kept::Done)
        };
        let done = run(env);
        if done.is_err() {
            env.exception_clear();
        }
        done
    });
    match done {
        Ok(kept) => kept,
        Err(error) => {
            crate::note(&format!("gallery: {error}"));
            Kept::Failed
        }
    }
}
