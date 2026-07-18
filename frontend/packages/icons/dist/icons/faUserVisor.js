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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserVisor.js
var require_faUserVisor = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserVisor.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-visor";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e04c";
    var svgPathData = "M343 175.2C324.2 222.5 278 256 224 256s-100.2-33.5-119-80.8c-14.3-3.2-25-16-25-31.2l0-64c0-17.7 14.3-32 32-32l12.1 0C147.5 18.7 183.6 0 224 0s76.5 18.7 99.9 48L336 48c17.7 0 32 14.3 32 32l0 64c0 15.3-10.7 28-25 31.2zM0 482.3C0 383.8 79.8 304 178.3 304l91.4 0C368.2 304 448 383.8 448 482.3c0 16.4-13.3 29.7-29.7 29.7L29.7 512C13.3 512 0 498.7 0 482.3zm348.7-19.9c2.4 3.2 7.3 1.4 7.3-2.5c-1.1-35.3-11.3-65.6-30.1-90.2c-1.2-2-4.6-1.9-6.1-.1c-19.9 23-31.9 53-35.8 89.5c-.3 3.5 3.7 5.5 6.5 3.5c1.8-1.4 4.1-3.4 6.6-5.6c9.2-8.1 22.3-19.5 31.7-19.5c2.3 2 7.6 7.8 19.9 25zM144 96c-8.8 0-16 7.2-16 16s7.2 16 16 16l160 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L144 96z";
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
    exports.faUserVisor = exports.definition;
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

// src/_entries/faUserVisor.js
var import_faUserVisor = __toESM(require_faUserVisor());
var export_faUserVisor = import_faUserVisor.definition;
export {
  export_faUserVisor as faUserVisor
};
