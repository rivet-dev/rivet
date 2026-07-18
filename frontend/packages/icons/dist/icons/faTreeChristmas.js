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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeChristmas.js
var require_faTreeChristmas = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTreeChristmas.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "tree-christmas";
    var width = 448;
    var height = 512;
    var aliases = [127876];
    var unicode = "f7db";
    var svgPathData = "M150.2 71l47.1-17.7L215 6.2c1.4-3.8 5-6.2 9-6.2s7.6 2.5 9 6.2l17.7 47.1L297.8 71c3.8 1.4 6.2 5 6.2 9s-2.5 7.6-6.2 9l-47.1 17.7L233 153.8c-1.4 3.8-5 6.2-9 6.2s-7.6-2.5-9-6.2l-17.7-47.1L150.2 89c-3.8-1.4-6.2-5-6.2-9s2.5-7.6 6.2-9zM263 165l11.8-31.5 144 180c3.4 4.2 5.2 9.4 5.2 14.8c0 13.1-10.6 23.7-23.7 23.7L368 352l76.2 122c2.5 4 3.8 8.5 3.8 13.2c0 13.7-11.1 24.9-24.9 24.9L24.9 512C11.1 512 0 500.9 0 487.1c0-4.7 1.3-9.2 3.8-13.2L80 352l-32.3 0C34.6 352 24 341.4 24 328.3c0-5.4 1.8-10.6 5.2-14.8l144-180L185 165c6.1 16.2 21.6 27 39 27s32.9-10.8 39-27zM184 288a24 24 0 1 0 -48 0 24 24 0 1 0 48 0zM288 440a24 24 0 1 0 0-48 24 24 0 1 0 0 48z";
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
    exports.faTreeChristmas = exports.definition;
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

// src/_entries/faTreeChristmas.js
var import_faTreeChristmas = __toESM(require_faTreeChristmas());
var export_faTreeChristmas = import_faTreeChristmas.definition;
export {
  export_faTreeChristmas as faTreeChristmas
};
