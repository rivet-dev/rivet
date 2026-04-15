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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDroneFront.js
var require_faDroneFront = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDroneFront.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "drone-front";
    var width = 640;
    var height = 512;
    var aliases = ["drone-alt"];
    var unicode = "f860";
    var svgPathData = "M155.7 112c-5.5-9.6-15.9-16-27.7-16s-22.2 6.4-27.7 16L24 112c-13.3 0-24 10.7-24 24s10.7 24 24 24l240 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-108.3 0zM376 112c-13.3 0-24 10.7-24 24s10.7 24 24 24l240 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-76.3 0c-5.5-9.6-15.9-16-27.7-16s-22.2 6.4-27.7 16L376 112zM96 192l0 64c0 12.6 7.4 24.1 19 29.2l56.5 25.1c-17.8 21.5-27.5 48.5-27.5 76.4l0 5.2c0 13.3 10.7 24 24 24s24-10.7 24-24l0-5.2c0-16.8 5.9-33.2 16.7-46.1l8.4-10.1L259 349.2c4.1 1.8 8.5 2.8 13 2.8l96 0c4.5 0 8.9-.9 13-2.8l41.9-18.6 8.4 10.1c10.8 12.9 16.7 29.3 16.7 46.1l0 5.2c0 13.3 10.7 24 24 24s24-10.7 24-24l0-5.2c0-27.9-9.7-54.9-27.5-76.4L525 285.2c11.6-5.1 19-16.6 19-29.2l0-64-64 0 0 43.2-17.8 7.9-40.6-23.7C390.7 201.5 355.7 192 320 192s-70.7 9.5-101.6 27.5l-40.6 23.7L160 235.2l0-43.2-64 0z";
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
    exports.faDroneFront = exports.definition;
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

// src/_entries/faDroneAlt.js
var import_faDroneFront = __toESM(require_faDroneFront());
var export_faDroneAlt = import_faDroneFront.definition;
export {
  export_faDroneAlt as faDroneAlt
};
