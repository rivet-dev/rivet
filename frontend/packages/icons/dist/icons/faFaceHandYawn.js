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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceHandYawn.js
var require_faFaceHandYawn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceHandYawn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-hand-yawn";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e379";
    var svgPathData = "M336.9 498.9C438.7 465.1 512 369.1 512 256C512 114.6 397.4 0 256 0S0 114.6 0 256c0 94.5 51.2 177 127.3 221.4c-9.8-18-15.3-38.6-15.3-60.5l0-32.8 0-53.6c0-28.7 23.3-52 52-52c1.8 0 3.6 .1 5.3 .3C184.8 246.3 217.8 224 256 224c28.7 0 54.5 12.6 72.1 32.6c14.8-2.4 30.4 2 41.8 13.4c12.2 12.2 16.5 29.4 12.8 45c1.1 .9 2.2 1.9 3.2 3c18.1 18.1 18.7 47.1 1.8 65.9c16.9 18.8 16.3 47.8-1.8 65.9l-49 49zM204.5 102c5.5 6.9 4.4 17-2.5 22.5l-80 64c-6.9 5.5-17 4.4-22.5-2.5s-4.4-17 2.5-22.5l80-64c6.9-5.5 17-4.4 22.5 2.5zM310 124.5c-6.9-5.5-8-15.6-2.5-22.5s15.6-8 22.5-2.5l80 64c6.9 5.5 8 15.6 2.5 22.5s-15.6 8-22.5 2.5l-80-64zM299.3 276.7c-6.2-6.2-16.4-6.2-22.6 0l-79 79c-5 5-13.7 1.5-13.7-5.7l0-19.7c0-11-9-20-20-20s-20 9-20 20l0 53.6 0 32.8c0 52.6 42.6 95.2 95.2 95.2c25.2 0 49.5-10 67.3-27.9l56.8-56.8c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L321.4 424c-2.6 2.6-6.8 2.6-9.4 0s-2.6-6.8 0-9.4l51.3-51.3c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L289.4 392c-2.6 2.6-6.8 2.6-9.4 0s-2.6-6.8 0-9.4l67.3-67.3c6.2-6.2 6.2-16.4 0-22.6s-16.4-6.2-22.6 0L257.4 360c-2.6 2.6-6.8 2.6-9.4 0s-2.6-6.8 0-9.4l51.3-51.3c6.2-6.2 6.2-16.4 0-22.6z";
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
    exports.faFaceHandYawn = exports.definition;
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

// src/_entries/faFaceHandYawn.js
var import_faFaceHandYawn = __toESM(require_faFaceHandYawn());
var export_faFaceHandYawn = import_faFaceHandYawn.definition;
export {
  export_faFaceHandYawn as faFaceHandYawn
};
