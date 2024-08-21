#[cfg(target_os = "linux")]
pub fn capture_screen() {
    use x11rb::{
        connection::Connection,
        protocol::xproto::{ ConnectionExt, Drawable, ImageFormat },
    };

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let screen = &conn.setup().roots[screen_num];

    let pixmap = conn.generate_id().unwrap();
    x11rb::protocol::xproto::create_pixmap(
        &conn,
        24,
        pixmap,
        screen.root,
        screen.width_in_pixels,
        screen.height_in_pixels
    );

    let image = conn.get_image(
        ImageFormat::Z_PIXMAP,
        pixmap,
        0,
        0,
        screen.width_in_pixels,
        screen.height_in_pixels,
        u32::MAX
    );

    dbg!(image);
}
use crate::error;

#[cfg(target_os = "windows")]
pub fn capture_screen(HTTP_MODE: bool) -> Result<Vec<u8>, error::GabinatorError> {
    use std::{ error::Error, fs::File, io::{ BufWriter, Cursor, Write }, time::Instant, u32 };
    use image::{
        codecs::{ jpeg::{ self, JpegEncoder }, png::{ PngDecoder, PngEncoder } },
        ColorType,
        DynamicImage,
        RgbImage,
        Rgba,
    };

    use std::{ io::Read, mem::size_of, ptr::{ null, null_mut } };
    use image::{ ExtendedColorType, ImageBuffer, ImageEncoder, ImageFormat, Rgb, RgbaImage };
    use sysinfo::System;
    use windows::{
        core::HRESULT,
        Win32::{
            Graphics::Gdi::{
                BitBlt,
                CreateCompatibleBitmap,
                CreateCompatibleDC,
                GetDC,
                GetDIBits,
                SelectObject,
                SetStretchBltMode,
                StretchBlt,
                BITMAPINFO,
                BITMAPINFOHEADER,
                BI_RGB,
                COLORONCOLOR,
                DIB_RGB_COLORS,
                SRCCOPY,
            },
            UI::WindowsAndMessaging::{
                GetDesktopWindow,
                GetSystemMetrics,
                SM_CXSCREEN,
                SM_CYSCREEN,
                SM_XVIRTUALSCREEN,
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
            SRCCOPY
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
            DIB_RGB_COLORS
        );

        //let swap_time = Instant::now();

        //De RGB a BGR, esto no se por que pero parece que tanto rust como c++ maneja este formato en vez de RGBA
        /*for px in buffer.chunks_exact_mut(3) {
            px.swap(0, 2);
        } */
        //println!("SWAP: {:.2?}", swap_time.elapsed());

        let encoding_time = Instant::now();
        let image = RgbImage::from_raw(width as u32, height as u32, buffer).expect(
            "Error convirtiendo en formato RGBA"
        );
        let jpeg = turbojpeg::compress_image(&image, 25, turbojpeg::Subsamp::Sub2x2).unwrap();
        //TESTS
        if HTTP_MODE{
            let mut file = File::create("temporal_image.jpg").unwrap();
            file.write_all(&jpeg);
        }
        
        //println!("ENCODING: {:.2?}", encoding_time.elapsed());
        //println!("TOTAL: {:.2?}", handler_time.elapsed());
        return Ok(jpeg.as_ref().to_vec());
    }
}
