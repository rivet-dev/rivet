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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD8.js
var require_faDiceD8 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiceD8.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice-d8";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6d2";
    var svgPathData = "M240 16c0-6.5-4-12.4-10-14.8s-13-1-17.5 3.7l-208 216c-3.7 3.9-5.3 9.3-4.1 14.6s4.9 9.5 9.9 11.5l208 80c4.9 1.9 10.5 1.2 14.8-1.7s6.9-7.9 6.9-13.2l0-296zM34.6 307.5c-4.1 6.3-3.3 14.6 1.9 20l176 179.7c4.5 4.6 11.5 6.1 17.5 3.6s10-8.3 10-14.8l0-112c0-6.6-4.1-12.6-10.3-14.9l-176-67.7c-7-2.7-15-.1-19.1 6.1zm442.7 0c-4.1-6.3-12.1-8.8-19.1-6.1l-176 67.7c-6.2 2.4-10.3 8.3-10.3 14.9l0 112c0 6.5 3.9 12.4 10 14.8s12.9 1 17.5-3.6l176-179.7c5.3-5.4 6.1-13.7 1.9-20zM282 1.2C276 3.6 272 9.5 272 16l0 296c0 5.3 2.6 10.2 6.9 13.2s9.9 3.6 14.8 1.7l208-80c5-1.9 8.7-6.2 9.9-11.5s-.4-10.7-4.1-14.6l-208-216C295 .2 288.1-1.3 282 1.2z";
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
    exports.faDiceD8 = exports.definition;
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

// src/_entries/faDiceD8.js
var import_faDiceD8 = __toESM(require_faDiceD8());
var export_faDiceD8 = import_faDiceD8.definition;
export {
  export_faDiceD8 as faDiceD8
};
