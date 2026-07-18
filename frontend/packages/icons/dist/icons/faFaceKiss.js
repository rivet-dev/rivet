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

// src/node_modules/@fortawesome/free-solid-svg-icons/faFaceKiss.js
var require_faFaceKiss = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faFaceKiss.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-kiss";
    var width = 512;
    var height = 512;
    var aliases = [128535, "kiss"];
    var unicode = "f596";
    var svgPathData = "M256 512a256 256 0 1 0 0-512 256 256 0 1 0 0 512zM240 288l32 0c26.5 0 48 21.5 48 48 0 12.3-4.6 23.5-12.2 32 7.6 8.5 12.2 19.7 12.2 32 0 26.5-21.5 48-48 48l-32 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-16 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l16 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-16 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zm-96-80a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm192-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faFaceKiss = exports.definition;
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

// src/_entries/faFaceKiss.js
var import_faFaceKiss = __toESM(require_faFaceKiss());
var export_faFaceKiss = import_faFaceKiss.definition;
export {
  export_faFaceKiss as faFaceKiss
};
