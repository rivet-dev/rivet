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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBadgerHoney.js
var require_faBadgerHoney = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBadgerHoney.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "badger-honey";
    var width = 640;
    var height = 512;
    var aliases = [129441];
    var unicode = "f6b4";
    var svgPathData = "M164.6 389.3l16-37.3 118.1 0 16.9 101.3c2.6 15.4 15.9 26.7 31.6 26.7l31.1 0c19.8 0 34.8-17.8 31.6-37.3L393 342.1c27.9-29 80.6-70.8 144.1-90.8l16.1 25.8c3.1 5 10.4 5 13.6 0l22.8-36.5c12.9-2.1 25.2-7.4 34.4-16.6c10.2-10.2 16-24.1 16-38.6l0-3.8c0-23.2-12.6-44.5-32.8-55.8L525.3 80.3C506.1 69.6 484.5 64 462.5 64c-20.1 0-39.9 4.7-57.8 13.6L400 80c-21 10.5-44.2 16-67.8 16L160 96l-32 0C57.3 96 0 153.3 0 224l0 32c0 17.7 14.3 32 32 32c5.4 0 10.6-1.4 15-3.7c9.1 17.1 22.1 31.9 37.7 43.3l-16.8 59c-2.5 8.8-2.5 18.2 .2 27l13.1 43.7c4.1 13.5 16.5 22.8 30.7 22.8l29.2 0c21.4 0 36.8-20.7 30.7-41.2l-8.3-27.7c-2.2-7.2-1.7-14.9 1.2-21.8zM570 160l-51.3 0c.9 2.5 1.4 5.2 1.4 8c0 13.3-10.7 24-24 24s-24-10.7-24-24c0-2.8 .5-5.5 1.4-8l-5.9 0c-23.2 0-46 6.3-65.9 18.2L294.8 242.3c-14.9 9-32 13.7-49.4 13.7L192 256c-46.4 0-86.2-28.2-103.2-68.5C102 161.7 129 144 160 144l172.2 0c31 0 61.5-7.2 89.2-21.1l4.7-2.4c11.3-5.6 23.7-8.6 36.3-8.6c13.8 0 27.4 3.5 39.5 10.2L570 160z";
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
    exports.faBadgerHoney = exports.definition;
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

// src/_entries/faBadgerHoney.js
var import_faBadgerHoney = __toESM(require_faBadgerHoney());
var export_faBadgerHoney = import_faBadgerHoney.definition;
export {
  export_faBadgerHoney as faBadgerHoney
};
