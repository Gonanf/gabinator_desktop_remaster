#[cfg(target_os = "macos")]
pub fn capture_screen() {}
#[cfg(target_os = "linux")]
pub fn capture_screen() {}
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
        let buffer_size = width * height * 4; //4 serian los valores R G B A por cada pixel
        let mut buffer = vec![0u8; buffer_size as usize];

        //Configuraciones
        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
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

        //De RGBA a BGRA, esto no se por que pero parece que tanto rust como c++ maneja este formato en vez de RGBA
        for px in buffer.chunks_exact_mut(4) {
            px.swap(0, 2);

            //verciones antiguas no soportan RGBA, entonces en esos casos el valor A sera reemplazado
            if
                px[3] == 0 &&
                System::os_version()
                    .map(|os_version| {
                        let strs: Vec<&str> = os_version.split(' ').collect();
                        strs[0].parse::<u8>().unwrap_or(0)
                    })
                    .unwrap_or(0) < 8
            {
                px[3] = 255;
            } 
        }
        Ok(buffer)

        /*let image = RgbaImage::from_raw(width as u32, height as u32, buffer).expect(
            "Error convirtiendo en formato RGBA"
        );

        image.save("Amongas.png").expect("Error guardando"); */
    }
}
