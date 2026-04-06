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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAmpGuitar.js
var require_faAmpGuitar = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAmpGuitar.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "amp-guitar";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f8a1";
    var svgPathData = "M256 0c-48.9 0-92.5 28.8-121.6 53.8c-15 12.9-27.3 25.7-35.8 35.3c-2.3 2.6-4.3 4.9-6 6.9L64 96C28.7 96 0 124.7 0 160L0 448c0 35.3 28.7 64 64 64l384 0c35.3 0 64-28.7 64-64l0-288c0-35.3-28.7-64-64-64l-28.5 0c-1.7-2.1-3.8-4.4-6-6.9c-8.5-9.6-20.8-22.4-35.8-35.3C348.5 28.8 304.9 0 256 0zm96.9 96L159.1 96c2.1-1.9 4.3-3.8 6.5-5.8C192.5 67.2 224.9 48 256 48s63.5 19.2 90.4 42.2c2.2 1.9 4.4 3.9 6.5 5.8zM96 152a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm72 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zm152-24a24 24 0 1 1 0 48 24 24 0 1 1 0-48zm72 24a24 24 0 1 1 48 0 24 24 0 1 1 -48 0zM64 448l0-192 384 0 0 192L64 448zm64-128a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm0 96a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm48-64a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm16-32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm16 80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm16-32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm48-64a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM256 416a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm48-64a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm16-32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm16 80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm16-32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm48-64a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM384 416a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faAmpGuitar = exports.definition;
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

// src/_entries/faAmpGuitar.js
var import_faAmpGuitar = __toESM(require_faAmpGuitar());
var export_faAmpGuitar = import_faAmpGuitar.definition;
export {
  export_faAmpGuitar as faAmpGuitar
};
