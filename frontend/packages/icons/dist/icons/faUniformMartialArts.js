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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUniformMartialArts.js
var require_faUniformMartialArts = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUniformMartialArts.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "uniform-martial-arts";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3d1";
    var svgPathData = "M180 0l28 0 9.6 0c12.3 0 23.5 7 28.8 18.1l61.1 126.7L253 288l-109 0 0-154.9L98 216.7c-1.3 2.4-2 5-2 7.7L96 384c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32L0 224.4c0-18.9 4.8-37.4 13.9-54L80.3 49.7C95.9 21.2 124.9 2.8 157 .3c1-.2 2-.3 3-.3l4.4 0L180 0zM389.1 20.6C393.8 8.2 405.7 0 419 0l13 0 28 0 15.6 0L480 0c1 0 2 .1 3 .3c32.1 2.5 61 20.9 76.7 49.5l66.4 120.7c9.1 16.5 13.9 35.1 13.9 54L640 384c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-159.6c0-2.7-.7-5.3-2-7.7l-46-83.7L496 288l-208.8 0 52.7-138.3L389.1 20.6zM320 320l176 0 0 32-107.6 0-38.5 0 47.4 71.1c4.9 7.4 2.9 17.3-4.4 22.2s-17.3 2.9-22.2-4.4l-50.7-76-50.7 76c-4.9 7.4-14.8 9.3-22.2 4.4s-9.3-14.8-4.4-22.2L290.1 352l-38.5 0L144 352l0-32 120 0 56 0zM216.1 405.4c-14.7 22.1-8.7 51.9 13.3 66.6s51.9 8.7 66.6-13.3L320 422.5l24.1 36.1c14.7 22.1 44.5 28 66.6 13.3s28-44.5 13.3-66.6L409.7 384l86.3 0 0 64c0 35.3-28.7 64-64 64l-224 0c-35.3 0-64-28.7-64-64l0-64 86.3 0-14.2 21.4z";
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
    exports.faUniformMartialArts = exports.definition;
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

// src/_entries/faUniformMartialArts.js
var import_faUniformMartialArts = __toESM(require_faUniformMartialArts());
var export_faUniformMartialArts = import_faUniformMartialArts.definition;
export {
  export_faUniformMartialArts as faUniformMartialArts
};
