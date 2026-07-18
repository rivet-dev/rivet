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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAperture.js
var require_faAperture = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAperture.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "aperture";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2df";
    var svgPathData = "M256 0c4 0 7.9 .1 11.9 .3c11.3 .5 17.5 12.9 11.8 22.8L177.5 200c-6.2 10.7-21.6 10.7-27.7 0L82.8 84.1c-3.6-6.3-2.6-14.3 2.9-19.2C130.9 24.5 190.6 0 256 0zM0 256c0-50.9 14.9-98.3 40.5-138.2c6.1-9.5 19.9-8.6 25.6 1.2L168.2 296c6.2 10.7-1.5 24-13.9 24L20.5 320c-7.3 0-13.7-4.9-15.2-12.1C1.8 291.2 0 273.8 0 256zM193.7 492c-3.6 6.3-11.1 9.4-18 7.1C111.8 478 59 432.5 28.5 373.6C23.3 363.5 31 352 42.3 352l204.4 0c12.3 0 20 13.3 13.9 24l-67 116zM256 512c-4 0-7.9-.1-11.9-.3c-11.3-.5-17.5-12.9-11.8-22.8L334.5 312c6.2-10.7 21.6-10.7 27.7 0l66.9 115.9c3.6 6.3 2.6 14.3-2.9 19.2C381.1 487.5 321.4 512 256 512zM512 256c0 50.9-14.9 98.3-40.5 138.2c-6.1 9.5-19.9 8.6-25.6-1.2L343.8 216c-6.2-10.7 1.5-24 13.9-24l133.9 0c7.3 0 13.7 4.9 15.2 12.1c3.5 16.8 5.3 34.1 5.3 51.9zM318.3 20c3.6-6.3 11.1-9.4 18-7.1C400.2 34 453 79.5 483.5 138.4c5.2 10.1-2.5 21.6-13.8 21.6l-204.4 0c-12.3 0-20-13.3-13.9-24l67-116z";
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
    exports.faAperture = exports.definition;
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

// src/_entries/faAperture.js
var import_faAperture = __toESM(require_faAperture());
var export_faAperture = import_faAperture.definition;
export {
  export_faAperture as faAperture
};
