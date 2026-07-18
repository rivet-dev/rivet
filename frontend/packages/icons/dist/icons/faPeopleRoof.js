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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPeopleRoof.js
var require_faPeopleRoof = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPeopleRoof.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "people-roof";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e537";
    var svgPathData = "M302.3-12.6c-9-4.5-19.6-4.5-28.6 0l-256 128C1.9 123.3-4.5 142.5 3.4 158.3s27.1 22.2 42.9 14.3L288 51.8 529.7 172.6c15.8 7.9 35 1.5 42.9-14.3s1.5-35-14.3-42.9l-256-128zM288 272a56 56 0 1 0 0-112 56 56 0 1 0 0 112zm0 48c-53 0-96 43-96 96l0 32c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32l0-32c0-53-43-96-96-96zM160 256a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zm352 0a48 48 0 1 0 -96 0 48 48 0 1 0 96 0zM112 336c-44.2 0-80 35.8-80 80l0 33.1c0 17 13.8 30.9 30.9 30.9l87.8 0c-4.3-9.8-6.7-20.6-6.7-32l0-48c0-18.4 3.5-36 9.8-52.2-12.2-7.5-26.5-11.8-41.8-11.8zM425.4 480l87.8 0c17 0 30.9-13.8 30.9-30.9l0-33.1c0-44.2-35.8-80-80-80-15.3 0-29.6 4.3-41.8 11.8 6.3 16.2 9.8 33.8 9.8 52.2l0 48c0 11.4-2.4 22.2-6.7 32z";
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
    exports.faPeopleRoof = exports.definition;
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

// src/_entries/faPeopleRoof.js
var import_faPeopleRoof = __toESM(require_faPeopleRoof());
var export_faPeopleRoof = import_faPeopleRoof.definition;
export {
  export_faPeopleRoof as faPeopleRoof
};
