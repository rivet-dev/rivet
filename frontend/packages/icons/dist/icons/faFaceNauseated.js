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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceNauseated.js
var require_faFaceNauseated = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceNauseated.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-nauseated";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e381";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zm79.6-336a32 32 0 1 1 0 64 32 32 0 1 1 0-64zm-128 32a32 32 0 1 1 -64 0 32 32 0 1 1 64 0zm-12.3-84.7l-7.4 7.4c-23.6 23.6-53.9 39.2-86.8 44.7l-2.5 .4c-8.7 1.5-17-4.4-18.4-13.2s4.4-17 13.2-18.4l2.5-.4c26.3-4.4 50.6-16.9 69.5-35.7l7.4-7.4c6.2-6.2 16.4-6.2 22.6 0s6.2 16.4 0 22.6zm121.4-22.6c6.2-6.2 16.4-6.2 22.6 0l7.4 7.4c18.9 18.9 43.2 31.4 69.5 35.7l2.5 .4c8.7 1.5 14.6 9.7 13.2 18.4s-9.7 14.6-18.4 13.2l-2.5-.4c-32.9-5.5-63.3-21.1-86.8-44.7l-7.4-7.4c-6.2-6.2-6.2-16.4 0-22.6zM168 282.7c19.5 11.3 33.8 30.5 38.4 53.3l99.2 0c4.6-22.8 18.9-42 38.4-53.3c7.7-4.4 17.4-1.8 21.9 5.8s1.8 17.4-5.8 21.9c-14.4 8.3-24 23.8-24 41.6s9.6 33.3 24 41.6c7.6 4.4 10.3 14.2 5.8 21.9s-14.2 10.3-21.9 5.8c-19.5-11.3-33.8-30.5-38.4-53.3l-99.2 0c-4.6 22.8-18.9 42-38.4 53.3c-7.6 4.4-17.4 1.8-21.9-5.8s-1.8-17.4 5.8-21.9c14.4-8.3 24-23.8 24-41.6s-9.6-33.3-24-41.6c-7.6-4.4-10.3-14.2-5.8-21.9s14.2-10.3 21.9-5.8z";
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
    exports.faFaceNauseated = exports.definition;
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

// src/_entries/faFaceNauseated.js
var import_faFaceNauseated = __toESM(require_faFaceNauseated());
var export_faFaceNauseated = import_faFaceNauseated.definition;
export {
  export_faFaceNauseated as faFaceNauseated
};
