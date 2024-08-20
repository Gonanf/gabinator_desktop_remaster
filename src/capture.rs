#[cfg(target_os = "linux")]
pub fn capture_screen() {
    use xcb::ffi::xproto::xcb_get_geometry_request_t;
    let (conn, _) = xcb::Connection::connect(None).unwrap();
    let screen = conn.get_setup().roots().next().unwrap();

    let pixmap = conn.generate_id();
    xcb::create_pixmap(&conn, 24, pixmap, screen.root(), screen.width_in_pixels(), screen.height_in_pixels());

    let image = conn.send_request(&x::GetImage {
        format: x::ImageFormat::ZPixmap,
        drawable: x::Drawable::Window(screen.root()),
        x: 0,
        y: 0,
        width,
        height,
        plane_mask: u32::MAX,});
    
    dbg!(image);


    xcb::free_pixmap(&conn, pixmap);
}
use std::{error::Error, fs::File, io::{BufWriter, Cursor, Write}, time::Instant, u32};

use image::{codecs::{jpeg::{self, JpegEncoder}, png::{PngDecoder, PngEncoder}}, ColorType, DynamicImage, RgbImage, Rgba};

#[cfg(target_os = "windows")]
use crate::error;
pub fn capture_screen() -> Result<Vec<u8>,error::GabinatorError> {
    use std::{ io::Read, mem::size_of, ptr::{ null, null_mut }};
    use image::{ ExtendedColorType, ImageBuffer, ImageEncoder, ImageFormat, Rgb, RgbaImage };
    use sysinfo::System;
    use windows::{core::HRESULT, Win32::{
        Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, GetDC, GetDIBits, SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLORONCOLOR, DIB_RGB_COLORS, SRCCOPY
        },
        UI::WindowsAndMessaging::{ GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN },
    }};

    use crate::error;
    //usando GDI
    //La mayoria de funciones de la api de windows son inseguras
    unsafe {
        let handler_time = Instant::now();

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
        StretchBlt(screen_copy, 0, 0, width, height, screen, xscreen, yscreen, width, height, SRCCOPY);

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

        let swap_time = Instant::now();

        //De RGB a BGR, esto no se por que pero parece que tanto rust como c++ maneja este formato en vez de RGBA
       for px in buffer.chunks_exact_mut(3) {
            px.swap(0, 2);
        } 
        println!("SWAP: {:.2?}",swap_time.elapsed());

        
        let encoding_time = Instant::now();
        let image = RgbImage::from_raw(width as u32, height as u32, buffer).expect(
            "Error convirtiendo en formato RGBA"
        );
        /*let mut new_buffer: Vec<u8> = Vec::new();
        let mut jpeg = JpegEncoder::new_with_quality(&mut new_buffer,10);
        jpeg.encode(&buffer, width as u32, height as u32, ExtendedColorType::Rgb8).unwrap(); */

        let jpeg = turbojpeg::compress_image(&image, 10, turbojpeg::Subsamp::Sub2x2).unwrap();
        //TESTS
        //let mut file = File::create("iconpo.png").unwrap();
        //file.write_all(&new_buffer);
        println!("ENCODING: {:.2?}",encoding_time.elapsed());
        println!("TOTAL: {:.2?}",handler_time.elapsed());
        return Ok(jpeg.as_ref().to_vec());
    }
}
