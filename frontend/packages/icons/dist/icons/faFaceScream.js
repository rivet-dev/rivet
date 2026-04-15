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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceScream.js
var require_faFaceScream = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceScream.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-scream";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e38b";
    var svgPathData = "M573.9 288.5c1.4-10.7 2.1-21.5 2.1-32.5C576 114.6 461.4 0 320 0S64 114.6 64 256c0 11 .7 21.9 2.1 32.6c20.6-2.5 41.7 3.5 57.8 16.9c0 0 0 0 0 0c57.5 48 87.8 118 98 187C252.1 505.1 285.2 512 320 512c34.8 0 68-6.9 98.2-19.5c10.2-69 40.4-139 98-187c0 0 0 0 0 0c16.1-13.4 37.2-19.4 57.8-17zM320 320c26.5 0 48 21.5 48 48l0 32c0 26.5-21.5 48-48 48s-48-21.5-48-48l0-32c0-26.5 21.5-48 48-48zm-46.6-77c-22.5 36.9-62.8 54.7-90.1 39.8s-31.1-56.9-8.7-93.7s62.8-54.7 90.1-39.8s31.1 56.9 8.7 93.7zm183.3 39.8c-27.3 14.9-67.6-2.9-90.1-39.8s-18.6-78.8 8.7-93.7s67.6 2.9 90.1 39.8s18.6 78.8-8.7 93.7zM103.4 330.1C89.1 318.2 68 316.4 51.6 327c-20 13-25.6 39.5-12.4 59.3c26.4 40 43.6 75.3 43.2 125.7L192 512c-7-68.4-34.8-137-88.7-181.9zM588.5 327c-16.4-10.6-37.6-8.8-51.8 3.1C482.8 375 455 443.6 448 512l109.8 0c-.4-50.5 16.7-85.7 43.2-125.7c13.1-19.8 7.6-46.3-12.4-59.3z";
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
    exports.faFaceScream = exports.definition;
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

// src/_entries/faFaceScream.js
var import_faFaceScream = __toESM(require_faFaceScream());
var export_faFaceScream = import_faFaceScream.definition;
export {
  export_faFaceScream as faFaceScream
};
