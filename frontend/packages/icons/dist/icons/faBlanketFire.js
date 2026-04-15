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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBlanketFire.js
var require_faBlanketFire = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBlanketFire.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "blanket-fire";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e3da";
    var svgPathData = "M0 128L0 352c0 70.7 57.3 128 128 128l209.6 0c-16.5-18.6-29.6-40.3-38.1-64L128 416c-35.3 0-64-28.7-64-64s28.7-64 64-64l173.8 0c6.3-15.5 14.4-30.8 23.3-45.3c21.7-35.2 51.9-71.2 87.3-102.8c10-8.9 23-12.8 35.5-11.7l0-.2c0-53-43-96-96-96L96 32C43 32 0 75 0 128zM288 350.1c0-9.8 1.3-19.9 3.6-30.1L128 320c-17.7 0-32 14.3-32 32s14.3 32 32 32l163 0c-2-11.1-3-22.4-3-33.9zM505.7 208.3l-19-19.9c-2-2.1-4-4.4-6-6.7c0 0 0 0 0 0c-9-10.2-18.6-21.3-32.7-21.6c-7.3-.2-14.6 2.3-20.3 7.5c-23.4 21.1-50 48.9-70.9 80.2C336 278.8 320 314.8 320 352.2C320 440.7 390.4 512 480 512c88.7 0 160-71.2 160-159.8c0-30-11-60.9-26.2-88.1c-15.2-27.4-35.3-52.3-55-70.6c-5.6-5.2-12.8-7.8-19.9-7.8c-7.6 0-15.5 2.8-20.9 8.9l-12.3 13.8zM544 400.2c0 35.3-28.7 64-64 64s-64-28.7-64-64c0-36.5 37-73 54.8-88.4c5.4-4.7 13.1-4.7 18.5 0C507 327.1 544 363.6 544 400.2z";
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
    exports.faBlanketFire = exports.definition;
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

// src/_entries/faBlanketFire.js
var import_faBlanketFire = __toESM(require_faBlanketFire());
var export_faBlanketFire = import_faBlanketFire.definition;
export {
  export_faBlanketFire as faBlanketFire
};
