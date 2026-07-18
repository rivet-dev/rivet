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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSmileHorns.js
var require_faFaceSmileHorns = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSmileHorns.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-smile-horns";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e391";
    var svgPathData = "M320 512c141.4 0 256-114.6 256-256c0-24.6-3.5-48.4-9.9-70.9c44.3-59.4 67.4-135 73.6-166c1.3-6.3-1.4-12.8-6.8-16.4s-12.4-3.6-17.8 0c-19.3 12.9-48.5 24.2-78.4 33.1c-23.3 7-46 12.2-63 15.5C430.9 19.1 377.7 0 320 0S209.1 19.1 166.2 51.3c-17-3.3-39.7-8.6-63-15.5C73.4 26.9 44.2 15.5 24.9 2.7C19.5-.9 12.5-.9 7.1 2.7S-1 12.8 .3 19.1c6.2 31 29.3 106.6 73.6 166C67.5 207.6 64 231.4 64 256c0 141.4 114.6 256 256 256zM228.1 349.5C246 370.2 276.6 392 320 392s74-21.8 91.9-42.5c5.8-6.7 15.9-7.4 22.6-1.6s7.4 15.9 1.6 22.6C413.8 396.1 375.1 424 320 424s-93.8-27.9-116.1-53.5c-5.8-6.7-5.1-16.8 1.6-22.6s16.8-5.1 22.6 1.6zM272.4 272c0 17.7-14.3 32-32 32s-32-14.3-32-32c0-8.8 3.6-16.8 9.3-22.6l-30.7-10.2c-8.4-2.8-12.9-11.9-10.1-20.2s11.9-12.9 20.2-10.1l96 32c8.4 2.8 12.9 11.9 10.1 20.2s-11.9 12.9-20.2 10.1l-10.9-3.6c.2 1.5 .3 2.9 .3 4.4zm128 32c-17.7 0-32-14.3-32-32c0-1.6 .1-3.2 .3-4.7l-11.7 3.9c-8.4 2.8-17.4-1.7-20.2-10.1s1.7-17.4 10.1-20.2l96-32c8.4-2.8 17.4 1.7 20.2 10.1s-1.7 17.4-10.1 20.2l-30.2 10.1c5.9 5.8 9.5 13.9 9.5 22.8c0 17.7-14.3 32-32 32z";
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
    exports.faFaceSmileHorns = exports.definition;
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

// src/_entries/faFaceSmileHorns.js
var import_faFaceSmileHorns = __toESM(require_faFaceSmileHorns());
var export_faFaceSmileHorns = import_faFaceSmileHorns.definition;
export {
  export_faFaceSmileHorns as faFaceSmileHorns
};
