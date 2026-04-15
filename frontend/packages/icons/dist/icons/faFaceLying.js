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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceLying.js
var require_faFaceLying = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceLying.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-lying";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e37e";
    var svgPathData = "M494 350.5c-4.5 1-9.2 1.5-14 1.5l-128 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l128 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-192 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l192 0c11.2 0 21.7 2.9 30.9 7.9C498.8 101.8 389.3 0 256 0C114.6 0 0 114.6 0 256S114.6 512 256 512c108 0 200.4-66.9 238-161.5zM144.4 176a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm160 0a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zM188.8 393.6c-5.3 7.1-15.3 8.5-22.4 3.2s-8.5-15.3-3.2-22.4C180.7 351.1 208.6 336 240 336c13.6 0 26.6 2.8 38.4 8c8.1 3.5 11.8 13 8.3 21.1s-13 11.8-21.1 8.3c-7.8-3.4-16.5-5.3-25.6-5.3c-20.9 0-39.5 10-51.2 25.6z";
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
    exports.faFaceLying = exports.definition;
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

// src/_entries/faFaceLying.js
var import_faFaceLying = __toESM(require_faFaceLying());
var export_faFaceLying = import_faFaceLying.definition;
export {
  export_faFaceLying as faFaceLying
};
