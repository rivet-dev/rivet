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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faMouseField.js
var require_faMouseField = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faMouseField.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "mouse-field";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e5a8";
    var svgPathData = "M396.4 112.9C398.8 105 400 96.6 400 88c0-48.6-39.4-88-88-88s-88 39.4-88 88c0 29.9 14.9 56.3 37.6 72.2C169.5 164.3 96 240.4 96 333.6c0 12.3 2.7 24 7.5 34.4L72 368c-39.8 0-72 32.2-72 72s32.2 72 72 72l384 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L72 464c-13.3 0-24-10.7-24-24s10.7-24 24-24l106.4 0 53.6 0 8 0 19 0 123.6 0c18.5 0 33.4-15 33.4-33.4c0-14.1-8.8-26.6-22-31.4l-43.7-15.9 23.8-63.5c2 .1 4 .2 6 .2l102.8 0c16.1 0 29.1-13 29.1-29.1c0-41.5-19.3-78.4-49.4-102.4C473.5 125.8 480 107.7 480 88c0-46.4-35.9-84.4-81.4-87.8C419.2 20.6 432 48.8 432 80c0 13.9-2.5 27.2-7.2 39.5c-9.1-3.2-18.6-5.5-28.4-6.6zM400 196a20 20 0 1 1 40 0 20 20 0 1 1 -40 0zM312 48c22.1 0 40 17.9 40 40l0 24c0 8.8-7.2 16-16 16l-24 0c-22.1 0-40-17.9-40-40s17.9-40 40-40z";
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
    exports.faMouseField = exports.definition;
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

// src/_entries/faMouseField.js
var import_faMouseField = __toESM(require_faMouseField());
var export_faMouseField = import_faMouseField.definition;
export {
  export_faMouseField as faMouseField
};
