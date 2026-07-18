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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointLeft.js
var require_faHandBackPointLeft = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHandBackPointLeft.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hand-back-point-left";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e19f";
    var svgPathData = "M0 184c0-22.1 17.9-40 40-40l200 0 7 0 73 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-55.7 0c1.4-2 3-3.9 4.5-5.8l16-19.2C297 72.4 315 64 334 64L372 64c38 0 74 16.9 98.3 46.1l4.5 5.4c24 28.8 37.1 65 37.1 102.4L512 336c0 70.7-57.3 128-128 128l-64 0-32 0c-26.5 0-48-21.5-48-48c0-12.4 4.7-23.6 12.3-32.1C227.5 382 208 361.3 208 336c0-12.4 4.7-23.6 12.3-32.1C195.5 302 176 281.3 176 256c0-12.3 4.6-23.5 12.2-32L40 224c-22.1 0-40-17.9-40-40z";
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
    exports.faHandBackPointLeft = exports.definition;
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

// src/_entries/faHandBackPointLeft.js
var import_faHandBackPointLeft = __toESM(require_faHandBackPointLeft());
var export_faHandBackPointLeft = import_faHandBackPointLeft.definition;
export {
  export_faHandBackPointLeft as faHandBackPointLeft
};
