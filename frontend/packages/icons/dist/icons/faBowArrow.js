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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBowArrow.js
var require_faBowArrow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBowArrow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bow-arrow";
    var width = 512;
    var height = 512;
    var aliases = [127993];
    var unicode = "f6b9";
    var svgPathData = "M511.7 18.9c1-5.2-.7-10.5-4.4-14.3S498.2-.7 493.1 .3l-128 24c-5.8 1.1-10.6 5.3-12.3 11s-.2 11.8 4 16l35 35L144.6 333.4 93.8 320.7c-8.2-2-16.8 .4-22.8 6.3L7 391c-6.1 6.1-8.5 15-6.2 23.4s8.9 14.8 17.3 16.9l50 12.5 12.5 50c2.1 8.4 8.5 15 16.9 17.3s17.3 0 23.4-6.2l64-64c6-6 8.4-14.6 6.3-22.8l-12.7-50.8L425.7 120.3l35 35c4.2 4.2 10.4 5.7 16 4s9.9-6.5 11-12.3l24-128zm-458 85.5c-12.6-11.6-32.1-11.3-44.3 .9c-12.5 12.5-12.5 32.8 0 45.3L145.8 287 191 241.8l-90.6-90.6c56.5-30.7 125.2-30.9 181.9-.7l46.8-46.8c-83.8-53.2-191.8-53-275.4 .8zM361.5 229.7c30.2 56.7 30 125.4-.7 181.9L270.2 321 225 366.2 361.4 502.6c12.5 12.5 32.8 12.5 45.3 0c12.2-12.2 12.5-31.8 .9-44.3c53.7-83.6 54-191.6 .8-275.4l-46.8 46.8z";
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
    exports.faBowArrow = exports.definition;
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

// src/_entries/faBowArrow.js
var import_faBowArrow = __toESM(require_faBowArrow());
var export_faBowArrow = import_faBowArrow.definition;
export {
  export_faBowArrow as faBowArrow
};
