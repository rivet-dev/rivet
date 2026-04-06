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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTireFlat.js
var require_faTireFlat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTireFlat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tire-flat";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f632";
    var svgPathData = "M256 32C114.6 32 0 146.6 0 288c0 47.3 12.8 91.7 35.2 129.7C14.9 423.3 0 441.9 0 464c0 26.5 21.5 48 48 48l416 0c26.5 0 48-21.5 48-48c0-22.1-14.9-40.7-35.2-46.3c22.4-38 35.2-82.4 35.2-129.7C512 146.6 397.4 32 256 32zm96 384c-2.1 1.6-4.3 3.2-6.6 4.7C319.9 437.9 289.1 448 256 448c-36 0-69.3-11.9-96-32c-38.9-29.2-64-75.7-64-128c0-88.4 71.6-160 160-160s160 71.6 160 160c0 52.3-25.1 98.8-64 128zm-96-96a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm16 95c44.3-5.5 81.5-33.6 99.8-72.4l-61.9-20.1c-8.6 13.5-22.1 23.5-38 27.6l0 65zM381.7 312.2c1.5-7.8 2.3-15.9 2.3-24.2c0-36.8-15.5-69.9-40.3-93.3l-38.2 52.6c9.1 11 14.5 25.2 14.5 40.6c0 1.4 0 2.7-.1 4l61.8 20.1zM256 224c8.3 0 16.3 1.6 23.6 4.5l38.2-52.6C299.5 165.8 278.4 160 256 160s-43.5 5.8-61.8 15.9l28.1 38.8 10 13.8c7.3-2.9 15.3-4.5 23.6-4.5zM128 288c0 8.3 .8 16.3 2.3 24.2L192.1 292c-.1-1.3-.1-2.7-.1-4c0-15.4 5.5-29.6 14.5-40.6l-10.1-13.9-28.1-38.7C143.5 218.1 128 251.2 128 288zm12.2 54.6c18.3 38.8 55.5 66.9 99.8 72.4l0-65c-15.8-4.1-29.3-14.1-37.9-27.5l-61.9 20.1z";
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
    exports.faTireFlat = exports.definition;
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

// src/_entries/faTireFlat.js
var import_faTireFlat = __toESM(require_faTireFlat());
var export_faTireFlat = import_faTireFlat.definition;
export {
  export_faTireFlat as faTireFlat
};
