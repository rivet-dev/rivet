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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFileXml.js
var require_faFileXml = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFileXml.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "file-xml";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e654";
    var svgPathData = "M0 64C0 28.7 28.7 0 64 0L224 0l0 128c0 17.7 14.3 32 32 32l128 0 0 144-208 0c-35.3 0-64 28.7-64 64l0 144-48 0c-35.3 0-64-28.7-64-64L0 64zm384 64l-128 0L256 0 384 128zM192 368c0 7.3 2.2 14.4 6.2 20.4l9.8 14.7 9.8-14.7c4-6.1 6.2-13.2 6.2-20.4c0-8.8 7.2-16 16-16s16 7.2 16 16c0 13.6-4 26.9-11.6 38.2L227.2 432l17.2 25.8C252 469.1 256 482.4 256 496c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-7.3-2.2-14.4-6.2-20.4L208 460.8l-9.8 14.7c-4 6.1-6.2 13.2-6.2 20.4c0 8.8-7.2 16-16 16s-16-7.2-16-16c0-13.6 4-26.9 11.6-38.2L188.8 432l-17.2-25.8C164 394.9 160 381.6 160 368c0-8.8 7.2-16 16-16s16 7.2 16 16zM448 496l0-128c0-8.8 7.2-16 16-16s16 7.2 16 16l0 112 16 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-32 0c-8.8 0-16-7.2-16-16zM299.7 352.6c6.9-1.9 14.3 1 18 7.2L352 416.9l34.3-57.1c3.7-6.2 11.1-9.1 18-7.2s11.7 8.2 11.7 15.4l0 128c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-70.2-18.3 30.5c-2.9 4.8-8.1 7.8-13.7 7.8s-10.8-2.9-13.7-7.8L320 425.8l0 70.2c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-128c0-7.2 4.8-13.5 11.7-15.4z";
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
    exports.faFileXml = exports.definition;
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

// src/_entries/faFileXml.js
var import_faFileXml = __toESM(require_faFileXml());
var export_faFileXml = import_faFileXml.definition;
export {
  export_faFileXml as faFileXml
};
