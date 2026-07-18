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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceMelting.js
var require_faFaceMelting = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceMelting.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-melting";
    var width = 640;
    var height = 512;
    var aliases = [129760];
    var unicode = "e483";
    var svgPathData = "M571.4 304.8c-4.5 23.2 11.8 47.2 35.4 47.2l1.3 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-48 0c-8.8 0-16 7.2-16 16s7.2 16 16 16l16 0c17.7 0 32 14.3 32 32s-14.3 32-32 32l-256 0L32 512c-17.7 0-32-14.3-32-32s14.3-32 32-32l12.9 0c36.8 0 58.4-50.7 42.9-84.1C72.5 331.1 64 294.5 64 256C64 114.6 178.6 0 320 0S576 114.6 576 256c0 16.7-1.6 33-4.6 48.8zM247.2 219.9a32 32 0 1 0 16.6-61.8 32 32 0 1 0 -16.6 61.8zM441 238.7a32 32 0 1 0 -61.8-16.6A32 32 0 1 0 441 238.7zM213.2 299.4c-3.9-7.9-13.4-11.3-21.4-7.4s-11.3 13.4-7.4 21.4c14.8 30.5 45.1 67.5 98.3 81.7s97.9-2.6 126-21.7c7.3-5 9.2-14.9 4.3-22.2s-14.9-9.2-22.2-4.3c-22.7 15.4-57.9 28.5-99.8 17.2s-65.8-40.2-77.8-64.8z";
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
    exports.faFaceMelting = exports.definition;
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

// src/_entries/faFaceMelting.js
var import_faFaceMelting = __toESM(require_faFaceMelting());
var export_faFaceMelting = import_faFaceMelting.definition;
export {
  export_faFaceMelting as faFaceMelting
};
