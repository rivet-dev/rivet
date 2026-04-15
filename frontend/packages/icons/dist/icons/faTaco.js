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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTaco.js
var require_faTaco = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTaco.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "taco";
    var width = 512;
    var height = 512;
    var aliases = [127790];
    var unicode = "f826";
    var svgPathData = "M274.8 38.4l44.1 34.2c5.6 4.4 12.6 6.6 19.7 6.4l55.8-1.7c14-.4 26.5 8.6 30.4 22.1l15.6 53.6c2 6.8 6.3 12.8 12.2 16.8L498.6 201c11.6 7.9 16.3 22.5 11.6 35.7l-17.1 47.7C441.1 209.2 354.3 160 256 160s-185.1 49.2-237.1 124.4L1.8 236.7c-4.7-13.1 0-27.8 11.6-35.7l46.1-31.4c5.9-4 10.2-9.9 12.2-16.8L87.3 99.3c3.9-13.4 16.4-22.5 30.4-22.1L173.4 79c7.1 .2 14.1-2 19.7-6.4l44.1-34.2c11-8.6 26.5-8.6 37.5 0zM512 448c0 17.7-14.3 32-32 32L32 480c-17.7 0-32-14.3-32-32C0 306.6 114.6 192 256 192s256 114.6 256 256zM112 416a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm80-80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0z";
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
    exports.faTaco = exports.definition;
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

// src/_entries/faTaco.js
var import_faTaco = __toESM(require_faTaco());
var export_faTaco = import_faTaco.definition;
export {
  export_faTaco as faTaco
};
