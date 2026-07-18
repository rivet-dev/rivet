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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSkiBoot.js
var require_faSkiBoot = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSkiBoot.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ski-boot";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3cc";
    var svgPathData = "M383.4 38c3.3-17.4-8-34.1-25.4-37.5s-34.1 8-37.5 25.4l-1.2 6-73.7 0c-22.1 130.8-76 201.3-127.2 238.9c-26.2 19.3-51.2 29.5-69.9 35c-6.3 1.8-11.8 3.1-16.5 4L.6 474c-1.8 9.4 .7 19.1 6.8 26.4S22.5 512 32 512l416 0 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-72c0-12.3-7-23.4-18-28.8L340 288l-68 0c-8.8 0-16-7.2-16-16s7.2-16 16-16l69.5 0 12.3-64L288 192c-8.8 0-16-7.2-16-16s7.2-16 16-16l72 0 17.3-90 6.2-32zM38.8 275.4c.3-.1 .6-.2 .9-.3c15.6-4.6 37.1-13.3 59.9-30c43-31.6 92.3-92.9 113.7-213.1L112 32C96.7 32 83.5 42.9 80.6 58L38.8 275.4z";
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
    exports.faSkiBoot = exports.definition;
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

// src/_entries/faSkiBoot.js
var import_faSkiBoot = __toESM(require_faSkiBoot());
var export_faSkiBoot = import_faSkiBoot.definition;
export {
  export_faSkiBoot as faSkiBoot
};
