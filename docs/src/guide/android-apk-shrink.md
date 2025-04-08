# Reduce APK size on Android

## Enable cargo release

The biggest reduction in the APK size comes from changing the `profile = "debug"`
to `profile = "release"` in the `shared/build.gradle` file

There is no need to set a release profile on the `typeGen` or `bindGen`


## Android minify


The following is more experimental and may take some trial and error to work.

[Enable minify](https://developer.android.com/studio/build/shrink-code)
in release mode in `app/build.gradle`
```diff
buildTypes {
    release {
-        minifyEnabled false
+        minifyEnabled true
        proguardFiles {
            getDefaultProguardFile('proguard-android-optimize.txt')
            'proguard-rules.pro'
        }
    }
}
```

Just enabling this feature will break your app as it will remove a lot of the
shared lib we depend on, to prevent this amend the `proguard-rules.pro` file to contain
the following

```
# There were a number of methods found in com.sun.jna that glued the android to
# the Rust the below is the most simplified way I could keep everything in
-keep class com.sun.**{
    static *; # put this in the below and the app breaks :D
}
-keep public class com.sun.jna.** {
    public final *;
    private protected *;
}
-keep class com.sun.jna.* {
  public protected *;
  public void read();
  public final *;
  * getTypeInfo() ;
}


# we want to keep all the shared library for conveiance.
# if you have some ios/other non android shared lib functions you may find it
# benifitial to exclude them here
-keep class <shared app package name>.** {
  public protected *;
}
```

If the above results in a crash at runtime you will need to expand the rules to
include more functions/classes, below is a set of links that can help with
understanding these rules.

<https://developer.android.com/build/shrink-code#keep-code>
<https://www.guardsquare.com/manual/configuration/examples>
<https://gendignoux.com/blog/2022/10/24/rust-library-android.html#shrinking-and-testing-the-release-apk>
