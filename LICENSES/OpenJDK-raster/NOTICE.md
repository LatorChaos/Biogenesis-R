# OpenJDK raster notices

The image raster port retains the Oracle notices in crates/core/src/image_alpha.rs.
That module uses GPL-2.0-only WITH Classpath-exception-2.0.
The combined Biogenesis-R package retains its existing GPL-2.0-only license.

This directory includes the retained OpenJDK LICENSE and ADDITIONAL_LICENSE_INFO.
Their SHA256 values are:

- jdk21-LICENSE: 4b9abebc4338048a7c2dc184e9f800deb349366bdf28eb23c2677a77b4c87726
- jdk21-ADDITIONAL_LICENSE_INFO: a69bce275ba7a3570af6579cb0f55682cd75fedfcd49e0e8e9022270c447c916

The compatibility authority is the inspected installed OpenJDK21 bytecode and native implementation.
The library hash is e34e9b6dcf0d040d137f2b9d49ed8540fb14186afea219e7c1c73f696a406745.
The java.desktop JMOD hash is 56d2162cfba58da0ddf89a55731ae46220e80fdb6700524e9dae604cb16d56d5.
These hashes establish the inspected artifacts, not universal JDK or platform compatibility.

Primary references retain their actual version attribution:

- [DMarlinRenderingEngine](https://raw.githubusercontent.com/openjdk/jdk21u/master/src/java.desktop/share/classes/sun/java2d/marlin/DMarlinRenderingEngine.java): JDK21u master retrieved 2026-10-27; no source commit pin
- [ShapeSpanIterator](https://raw.githubusercontent.com/openjdk/jdk21u/master/src/java.desktop/share/native/libawt/java2d/pipe/ShapeSpanIterator.c): JDK21u master retrieved 2026-10-27; no source commit pin
- [Stroker](https://raw.githubusercontent.com/openjdk/jdk/master/src/java.desktop/share/classes/sun/java2d/marlin/Stroker.java): OpenJDK mainline, not installed21 source
- [AlphaMath](https://raw.githubusercontent.com/openjdk/jdk8u/master/jdk/src/share/native/sun/java2d/loops/AlphaMath.c): OpenJDK8u master, not installed21 source

AlphaMath carries Copyright (c) 2000, Oracle and/or its affiliates. All rights reserved.
The port expresses its byte rounding arithmetic without copying macro bodies or tables.
Read [ALPHA_SOURCE_MAP.md](../../ALPHA_SOURCE_MAP.md) for the derivation and its limits.
