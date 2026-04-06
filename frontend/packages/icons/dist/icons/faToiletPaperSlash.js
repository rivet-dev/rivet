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

// src/node_modules/@fortawesome/free-solid-svg-icons/faToiletPaperSlash.js
var require_faToiletPaperSlash = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faToiletPaperSlash.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "toilet-paper-slash";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e072";
    var svgPathData = "M41-24.9c-9.4-9.4-24.6-9.4-33.9 0S-2.3-.3 7 9.1l528 528c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9l-94.5-94.5c40.2-23 69.5-96.9 69.5-184.6 0-106-43-192-96-192s-96 86-96 192c0 24.1 2.2 47.2 6.3 68.4L304 238.2 304 224c0-58.9 11.8-114.5 33.2-157.2 5.9-11.7 13.1-23.7 21.9-34.8L128 32c-8.3 0-16.4 2.1-24.1 6.1l-63-63zM32 224l0 264c0 30.9 25.1 56 56 56l160 0c30.9 0 56-25.1 56-56l0-114.2-256-256C37.9 148.2 32 184.7 32 224zm416 64c-17.7 0-32-28.7-32-64s14.3-64 32-64 32 28.7 32 64-14.3 64-32 64z";
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
    exports.faToiletPaperSlash = exports.definition;
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

// src/_entries/faToiletPaperSlash.js
var import_faToiletPaperSlash = __toESM(require_faToiletPaperSlash());
var export_faToiletPaperSlash = import_faToiletPaperSlash.definition;
export {
  export_faToiletPaperSlash as faToiletPaperSlash
};
