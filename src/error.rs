
pub enum GabinatorError{
    UsbError(String),
    CaptureError(String),
    MainError(String),
}

static DEBUG_ERROR: bool = true;
//Make Connection error

impl GabinatorError {
    pub fn newUSB<S: ToString>(message: S) -> Self{
        if DEBUG_ERROR{
            dbg!(message.to_string());
        }  
        GabinatorError::UsbError(message.to_string())  
    }

    pub fn newCapture<S: ToString>(message: S) -> Self{
        if DEBUG_ERROR{
            dbg!(message.to_string());
        }  
        GabinatorError::CaptureError(message.to_string())    
    }

    pub fn newMain<S: ToString>(message: S) -> Self{
        if DEBUG_ERROR{
            dbg!(message.to_string());
        }  
        GabinatorError::MainError(message.to_string())    
    }
}
