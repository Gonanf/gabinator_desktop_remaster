#[cfg(target_os = "linux")]
pub fn capture_screen(quality: u8) -> xcb::Result<Vec<u8>> {
    use std::u32;

    use image::RgbaImage;
    use xcb::{
        x::{self, GetImage, ImageOrder},
        Connection,
    };
    let (conn, window_number) = Connection::connect(None)?;

    let setup = conn.get_setup();
    let mut window = setup.roots();
    let current_window = window.next().unwrap();
    let width = current_window.width_in_pixels() as u32;
    let height = current_window.height_in_pixels() as u32;

    let cookie_get_image = conn.send_request(&GetImage {
        format: x::ImageFormat::ZPixmap,
        drawable: x::Drawable::Window(current_window.root()),
        x: 0,
        y: 0,
        width: width as u16,
        height: height as u16,
        plane_mask: u32::MAX,
    });
    let image_reply = conn.wait_for_reply(cookie_get_image).unwrap();
    let data = image_reply.data();
    let depth = image_reply.depth();
    let pixmap = setup
        .pixmap_formats()
        .iter()
        .find(|i| i.depth() == depth)
        .unwrap();

    let bits_per_pixel = pixmap.bits_per_pixel();
    let bit_order = setup.bitmap_format_bit_order();
    let mut image_data = vec![0u8; (width * height * 4) as usize];
    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * bits_per_pixel as u32 / 8) as usize;
            let (r, g, b, a) = match depth {
                8 => {
                    let pixel = if bit_order == ImageOrder::LsbFirst {
                        data[index]
                    } else {
                        data[index] & 7 << 4 | data[index] >> 4
                    };

                    (
                        ((pixel >> 6) as f32 / 3.0 * 255.0) as u8,
                        (((pixel >> 2) & 7) as f32 / 7.0 * 255.0) as u8,
                        ((pixel & 3) as f32 / 3.0 * 255.0) as u8,
                        255 as u8,
                    )
                }

                16 => {
                    let pixel = if bit_order == ImageOrder::LsbFirst {
                        data[index] as u16 | (data[index + 1] as u16) << 8
                    } else {
                        (data[index] as u16) << 8 | data[index + 1] as u16
                    };

                    (
                        ((pixel >> 11) as f32 / 31.0 * 255.0) as u8,
                        (((pixel >> 5) & 63) as f32 / 63.0 * 255.0) as u8,
                        ((pixel & 31) as f32 / 31.0 * 255.0) as u8,
                        255 as u8,
                    )
                }

                24 | 32 => {
                    if bit_order == ImageOrder::LsbFirst {
                        (data[index + 2], data[index + 1], data[index], 255 as u8)
                    } else {
                        (data[index], data[index + 1], data[index + 2], 255 as u8)
                    }
                }

                _ => (0 as u8, 0 as u8, 0 as u8, 0 as u8),
            };

            let local = ((y * width + x) * 4) as usize;
            image_data[local] = r;
            image_data[local + 1] = g;
            image_data[local + 2] = b;
            image_data[local + 3] = a;
        }
    }
    let image = RgbaImage::from_raw(width.into(), height.into(), image_data.clone()).unwrap();
    let jpeg = turbojpeg::compress_image(&image, quality.into(), turbojpeg::Subsamp::Sub2x2).unwrap();
    dbg!(jpeg.len());
    return Ok(jpeg.as_ref().to_vec());
}

#[cfg(target_os = "windows")]
pub fn capture_screen(quality: u8) -> Result<Vec<u8>, error::GabinatorError> {
    use image::{
        codecs::{
            jpeg::{self, JpegEncoder},
            png::{PngDecoder, PngEncoder},
        },
        ColorType, DynamicImage, RgbImage, Rgba,
    };
    use std::{
        error::Error,
        fs::File,
        io::{BufWriter, Cursor, Write},
        time::Instant,
        u32,
    };

    use image::{ExtendedColorType, ImageBuffer, ImageEncoder, ImageFormat, Rgb, RgbaImage};
    use std::{
        io::Read,
        mem::size_of,
        ptr::{null, null_mut},
    };
    use sysinfo::System;
    use windows::{
        core::HRESULT,
        Win32::{
            Graphics::Gdi::{
                BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, GetDC, GetDIBits, SelectObject,
                SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLORONCOLOR,
                DIB_RGB_COLORS, SRCCOPY,
            },
            UI::WindowsAndMessaging::{
                GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_XVIRTUALSCREEN,
                SM_YVIRTUALSCREEN,
            },
        },
    };

    //usando GDI
    //La mayoria de funciones de la api de windows son inseguras
    unsafe {
        //let handler_time = Instant::now();

        //Obtener resolucion
        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        let xscreen = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let yscreen = GetSystemMetrics(SM_YVIRTUALSCREEN);

        //Obtener un handler del device context de la pantalla completa
        let window = GetDesktopWindow();
        let screen = GetDC(window);

        //Iniciar una variable que sera una copia de la pantalla (Para no modificar el original en pasos siguientes)
        let screen_copy = CreateCompatibleDC(screen);

        //Crear un bitmap para alojar lo que contenga la copia de la pantalla
        //Aca me equivoque anteriormente, puse sobre crear un bitmap para la copia de la pantalla, pero la copia aun no tenia datos
        let mut bitmap = CreateCompatibleBitmap(screen, width, height);

        //Unir el bitmap con la copia de la pantalla  y luego copiar el contenido de la pantalla a la copia
        SelectObject(screen_copy, bitmap);
        SetStretchBltMode(screen_copy, COLORONCOLOR);
        StretchBlt(
            screen_copy,
            0,
            0,
            width,
            height,
            screen,
            xscreen,
            yscreen,
            width,
            height,
            SRCCOPY,
        );

        //Crear un buffer que contendra el bitmap del header bitmap
        let buffer_size = width * height * 3; //4 serian los valores R G B A por cada pixel
        let mut buffer = vec![0u8; buffer_size as usize];

        //Configuraciones
        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 24,
                biSizeImage: buffer_size as u32,
                biCompression: 0,
                ..Default::default()
            },
            ..Default::default()
        };

        //Extraer el bitmap
        GetDIBits(
            screen_copy,
            bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        );

        //De RGB a BGR, si no se pone asi, el rojo y el azul se ven intercambiados en la imagen final
        for px in buffer.chunks_exact_mut(3) {
            px.swap(0, 2);
        }

        let encoding_time = Instant::now();
        let image = RgbImage::from_raw(width as u32, height as u32, buffer)
            .expect("Error convirtiendo en formato RGBA");
        let jpeg = turbojpeg::compress_image(&image, quality.into(), turbojpeg::Subsamp::Sub2x2).unwrap();
        return Ok(jpeg.as_ref().to_vec());
    }
}
