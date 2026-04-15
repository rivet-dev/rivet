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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLips.js
var require_faLips = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLips.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lips";
    var width = 576;
    var height = 512;
    var aliases = [128068, 128482];
    var unicode = "f600";
    var svgPathData = "M288 101.3c3-2.4 6.2-4.8 9.7-7.3c17.8-12.7 46.8-30 78.3-30c20.3 0 42.8 9.3 61.4 19.2c20 10.6 41.1 24.7 60.2 39.5c19.1 14.8 37.1 31.2 50.8 46.5c6.8 7.6 13.1 15.7 17.9 23.7c4.3 7.2 9.7 18.3 9.7 31c0 9.2-2.6 19.1-5.3 27.5c-3 9.1-7.4 19.4-12.8 30.3c-10.8 21.7-26.8 46.9-47.7 71.1C468.8 400.9 404.7 448 320 448l-64 0c-84.7 0-148.8-47.1-190.2-95.1C44.9 328.8 29 303.5 18.1 281.8C12.7 271 8.4 260.7 5.3 251.5C2.6 243.1 0 233.2 0 224c0-12.8 5.5-23.8 9.7-31c4.8-8.1 11.1-16.1 17.9-23.7c13.6-15.3 31.7-31.7 50.8-46.5c19.1-14.9 40.2-29 60.2-39.5C157.2 73.3 179.7 64 200 64c31.5 0 60.6 17.2 78.3 30c3.4 2.5 6.7 4.9 9.7 7.3zM64 224c0 7.9 80 96 224 96s224-88.1 224-96c0-7.7-46.3-31.1-138.8-32c-3.4 0-6.9 .5-10.1 1.5C350 197.6 313.7 208 288 208s-62-10.4-75.1-14.4c-3.3-1-6.7-1.6-10.1-1.5C110.3 192.9 64 216.3 64 224z";
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
    exports.faLips = exports.definition;
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

// src/_entries/faLips.js
var import_faLips = __toESM(require_faLips());
var export_faLips = import_faLips.definition;
export {
  export_faLips as faLips
};
