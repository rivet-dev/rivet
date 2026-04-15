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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceZany.js
var require_faFaceZany = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceZany.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-zany";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3a4";
    var svgPathData = "M384.1 477.7C506.5 407 548.5 250.4 477.8 128S250.5-36.4 128.1 34.3S-36.3 261.6 34.4 384c36.2 62.7 94.9 104.3 160.1 120.5c-11.5-15.8-18.4-35.4-18.4-56.5l0-39.8c-10.4-6.2-20.1-13.7-28.9-22.2c-10.2-9.9-6.6-26.3 5.7-33.4L391.2 214.9c12.3-7.1 28.3-2 31.8 11.8c15.2 59.4-5.9 124.1-55 165l0 56.3c0 15.4-3.6 30-10.1 42.9c8.8-3.8 17.5-8.2 26.1-13.2zM128 160a64 64 0 1 1 0 128 64 64 0 1 1 0-128zm112-8a72 72 0 1 1 144 0 72 72 0 1 1 -144 0zm96.1 296l0-45.4c0-14.7-11.9-26.6-26.6-26.6l-2 0c-11.3 0-21.1 7.9-23.6 18.9c-2.8 12.6-20.8 12.6-23.6 0C257.8 383.9 248 376 236.7 376l-2 0c-14.7 0-26.6 11.9-26.6 26.6l0 45.4c0 35.3 28.7 64 64 64s64-28.7 64-64zM152 224a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zm144-56a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faFaceZany = exports.definition;
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

// src/_entries/faFaceZany.js
var import_faFaceZany = __toESM(require_faFaceZany());
var export_faFaceZany = import_faFaceZany.definition;
export {
  export_faFaceZany as faFaceZany
};
