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

// src/node_modules/@fortawesome/free-solid-svg-icons/faVenusMars.js
var require_faVenusMars = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faVenusMars.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "venus-mars";
    var width = 640;
    var height = 512;
    var aliases = [9892];
    var unicode = "f228";
    var svgPathData = "M480-64c-17.7 0-32 14.3-32 32S462.3 0 480 0L530.7 0 474 56.7c-26.3-15.7-57.1-24.7-90-24.7-35.4 0-68.4 10.5-96 28.5-27.6-18-60.6-28.5-96-28.5-97.2 0-176 78.8-176 176 0 86.3 62.1 158.1 144 173.1l0 34.9-32 0c-17.7 0-32 14.3-32 32s14.3 32 32 32l32 0 0 32c0 17.7 14.3 32 32 32s32-14.3 32-32l0-32 32 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32 0 0-34.9c23.3-4.3 44.9-13.1 64-25.6 27.6 18 60.6 28.5 96 28.5 97.2 0 176-78.8 176-176 0-41.1-14.1-79-37.8-109L576 45.3 576 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-128c0-17.7-14.3-32-32-32L480-64zM336 309.2c20.2-28.6 32-63.5 32-101.2s-11.8-72.6-32-101.2c14.6-6.9 30.8-10.8 48-10.8 61.9 0 112 50.1 112 112S445.9 320 384 320c-17.2 0-33.5-3.9-48-10.8zM288 150.3c10.2 16.9 16 36.6 16 57.7s-5.8 40.9-16 57.7c-10.2-16.9-16-36.6-16-57.7s5.8-40.9 16-57.7zm-48-43.5c-20.2 28.6-32 63.5-32 101.2s11.8 72.6 32 101.2c-14.5 6.9-30.8 10.8-48 10.8-61.9 0-112-50.1-112-112S130.1 96 192 96c17.2 0 33.5 3.9 48 10.8z";
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
    exports.faVenusMars = exports.definition;
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

// src/_entries/faVenusMars.js
var import_faVenusMars = __toESM(require_faVenusMars());
var export_faVenusMars = import_faVenusMars.definition;
export {
  export_faVenusMars as faVenusMars
};
