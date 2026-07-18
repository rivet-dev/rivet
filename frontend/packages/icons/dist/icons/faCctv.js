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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraCctv.js
var require_faCameraCctv = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCameraCctv.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-cctv";
    var width = 576;
    var height = 512;
    var aliases = ["cctv"];
    var unicode = "f8ac";
    var svgPathData = "M140.3 2.5c-14.9-6.2-32.1-.3-40.1 13.7l-96 168c-4.6 8-5.5 17.5-2.5 26.2s9.5 15.7 18 19.2l158.1 65.9L137.9 400 64 400l0-16c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 48 0 48c0 17.7 14.3 32 32 32s32-14.3 32-32l0-16 96 0c13.3 0 25.2-8.2 29.9-20.6l47-123.4 70.8 29.5c9.9 4.1 21.2 3 30.1-2.9l192-128c9.8-6.5 15.2-18 14.1-29.7s-8.6-21.9-19.5-26.5l-384-160zM513.4 398.6c3.9 1.8 8.4 1.9 12.5 .3s7.2-4.7 8.9-8.7l40-96c3.3-8-.3-17.2-8.3-20.8l-36.7-16.3L396.9 345.6l116.5 52.9z";
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
    exports.faCameraCctv = exports.definition;
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

// src/_entries/faCctv.js
var import_faCameraCctv = __toESM(require_faCameraCctv());
var export_faCctv = import_faCameraCctv.definition;
export {
  export_faCctv as faCctv
};
