var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBarcodeRead.js
var require_faBarcodeRead = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBarcodeRead.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "barcode-read";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f464";
    var svgPathData = "M56 48c-4.4 0-8 3.6-8 8l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24L0 56C0 25.1 25.1 0 56 0l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24L56 48zm96 80l16 0c13.3 0 24 10.7 24 24l0 208c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-208c0-13.3 10.7-24 24-24zm160 0l16 0c13.3 0 24 10.7 24 24l0 208c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-208c0-13.3 10.7-24 24-24zm72 24c0-13.3 10.7-24 24-24l16 0c13.3 0 24 10.7 24 24l0 208c0 13.3-10.7 24-24 24l-16 0c-13.3 0-24-10.7-24-24l0-208zM240 128c8.8 0 16 7.2 16 16l0 224c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-224c0-8.8 7.2-16 16-16zM528 56c0-4.4-3.6-8-8-8l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l80 0c30.9 0 56 25.1 56 56l0 80c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-80zM56 464l80 0c13.3 0 24 10.7 24 24s-10.7 24-24 24l-80 0c-30.9 0-56-25.1-56-56l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24l0 80c0 4.4 3.6 8 8 8zm472-8l0-80c0-13.3 10.7-24 24-24s24 10.7 24 24l0 80c0 30.9-25.1 56-56 56l-80 0c-13.3 0-24-10.7-24-24s10.7-24 24-24l80 0c4.4 0 8-3.6 8-8z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faBarcodeRead = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faBarcodeRead.js
var import_faBarcodeRead = __toESM(require_faBarcodeRead());
var export_faBarcodeRead = import_faBarcodeRead.definition;
export {
  export_faBarcodeRead as faBarcodeRead
};
