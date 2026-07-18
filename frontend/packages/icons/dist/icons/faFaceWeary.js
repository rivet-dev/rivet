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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceWeary.js
var require_faFaceWeary = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceWeary.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-weary";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e3a1";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM164.7 328.7c22-22 53.9-40.7 91.3-40.7s69.3 18.7 91.3 40.7c11.1 11.1 20.1 23.4 26.4 35.4c6.2 11.7 10.3 24.4 10.3 35.9c0 5.2-2.6 10.2-6.9 13.2s-9.8 3.7-14.7 1.8l-20.5-7.7c-26.9-10.1-55.5-15.3-84.3-15.3l-3.2 0c-28.8 0-57.3 5.2-84.3 15.3L149.6 415c-4.9 1.8-10.4 1.2-14.7-1.8s-6.9-7.9-6.9-13.2c0-11.6 4.2-24.2 10.3-35.9c6.3-12 15.3-24.3 26.4-35.4zm46.6-164c6.2 6.2 6.2 16.4 0 22.6l-7.4 7.4c-23.6 23.6-53.9 39.2-86.8 44.7l-2.5 .4c-8.7 1.5-17-4.4-18.4-13.2s4.4-17 13.2-18.4l2.5-.4c26.3-4.4 50.6-16.9 69.5-35.7l7.4-7.4c6.2-6.2 16.4-6.2 22.6 0zm89.4 22.6c-6.2-6.2-6.2-16.4 0-22.6s16.4-6.2 22.6 0l7.4 7.4c18.9 18.9 43.2 31.4 69.5 35.7l2.5 .4c8.7 1.5 14.6 9.7 13.2 18.4s-9.7 14.6-18.4 13.2l-2.5-.4c-32.9-5.5-63.3-21.1-86.8-44.7l-7.4-7.4z";
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
    exports.faFaceWeary = exports.definition;
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

// src/_entries/faFaceWeary.js
var import_faFaceWeary = __toESM(require_faFaceWeary());
var export_faFaceWeary = import_faFaceWeary.definition;
export {
  export_faFaceWeary as faFaceWeary
};
