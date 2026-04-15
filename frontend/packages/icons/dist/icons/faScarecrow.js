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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScarecrow.js
var require_faScarecrow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScarecrow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scarecrow";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f70d";
    var svgPathData = "M320 96c0 11.2-1.9 22-5.5 32L416 128c5.9 0 11.3 3.2 14.1 8.5s2.5 11.5-.8 16.4l-7.6 11.4 19.9 14.9c4 3 6.4 7.8 6.4 12.8s-2.4 9.8-6.4 12.8l-19.9 14.9 7.6 11.4c3.3 4.9 3.6 11.2 .8 16.4s-8.2 8.5-14.1 8.5l-73 0c-10.1 0-17.7 9.2-15.7 19.1l24.3 121.7c1.2 6.2-1.2 12.5-6.3 16.1s-11.8 4-17.3 .9L287.9 391c-4.9-2.8-11-2.8-15.9 0l-40.1 22.9c-4.9 2.8-11 2.8-15.9 0L175.9 391c-4.9-2.8-11-2.8-15.9 0l-40.1 22.9c-5.4 3.1-12.2 2.8-17.3-.9s-7.6-10-6.3-16.1l24.3-121.7c2-9.9-5.6-19.1-15.7-19.1l-73 0c-5.9 0-11.3-3.2-14.1-8.5s-2.5-11.5 .8-16.4l7.6-11.4L6.4 204.8C2.4 201.8 0 197 0 192s2.4-9.8 6.4-12.8l19.9-14.9-7.6-11.4c-3.3-4.9-3.6-11.2-.8-16.4s8.2-8.5 14.1-8.5l101.5 0c-3.5-10-5.5-20.8-5.5-32c0-53 43-96 96-96s96 43 96 96zM208 80a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zm48 32a16 16 0 1 0 0-32 16 16 0 1 0 0 32zM192 480l0-52.2 12.2 6.9c12.3 7 27.4 7 39.7 0l12.2-6.9 0 52.2c0 17.7-14.3 32-32 32s-32-14.3-32-32z";
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
    exports.faScarecrow = exports.definition;
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

// src/_entries/faScarecrow.js
var import_faScarecrow = __toESM(require_faScarecrow());
var export_faScarecrow = import_faScarecrow.definition;
export {
  export_faScarecrow as faScarecrow
};
