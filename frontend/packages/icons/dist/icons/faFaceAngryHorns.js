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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceAngryHorns.js
var require_faFaceAngryHorns = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceAngryHorns.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-angry-horns";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e368";
    var svgPathData = "M576 256c0 141.4-114.6 256-256 256S64 397.4 64 256c0-24.6 3.5-48.4 9.9-70.9C29.6 125.8 6.5 50.1 .3 19.1C-1 12.8 1.7 6.3 7.1 2.7s12.4-3.6 17.8 0c19.3 12.9 48.5 24.2 78.4 33.1c23.3 7 46 12.2 63 15.5C209.1 19.1 262.3 0 320 0s110.9 19.1 153.8 51.3c17-3.3 39.7-8.6 63-15.5c29.8-8.9 59.1-20.2 78.4-33.1c5.4-3.6 12.4-3.6 17.8 0s8.1 10.1 6.8 16.4c-6.2 31-29.3 106.6-73.6 166c6.5 22.5 9.9 46.3 9.9 70.9zM402.7 411.9c6.6-5.9 7.1-16 1.2-22.6C387.8 371.4 359.7 352 320 352s-67.8 19.4-83.9 37.3c-5.9 6.6-5.4 16.7 1.2 22.6s16.7 5.4 22.6-1.2c11.7-13 31.6-26.7 60.1-26.7s48.4 13.7 60.1 26.7c5.9 6.6 16 7.1 22.6 1.2zM240.4 296c13.3 0 24-10.7 24-24c0-2.6-.4-5.1-1.2-7.4l19.7 6.6c8.4 2.8 17.4-1.7 20.2-10.1s-1.7-17.4-10.1-20.2l-96-32c-8.4-2.8-17.4 1.7-20.2 10.1s1.7 17.4 10.1 20.2l39.6 13.2c-6.2 4.3-10.2 11.5-10.2 19.6c0 13.3 10.7 24 24 24zm184-24c0-8.2-4.1-15.5-10.4-19.8l39.1-13c8.4-2.8 12.9-11.9 10.1-20.2s-11.9-12.9-20.2-10.1l-96 32c-8.4 2.8-12.9 11.9-10.1 20.2s11.9 12.9 20.2 10.1l20.6-6.9c-.8 2.4-1.3 5-1.3 7.7c0 13.3 10.7 24 24 24s24-10.7 24-24z";
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
    exports.faFaceAngryHorns = exports.definition;
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

// src/_entries/faFaceAngryHorns.js
var import_faFaceAngryHorns = __toESM(require_faFaceAngryHorns());
var export_faFaceAngryHorns = import_faFaceAngryHorns.definition;
export {
  export_faFaceAngryHorns as faFaceAngryHorns
};
